mod aws;
mod cli;
mod config;
mod proxy;
mod state;
mod store;

use anyhow::{bail, Result};
use chrono::Utc;
use clap::{Parser, ValueEnum};
use cli::{Cli, Commands, ConfigAction, UnsetOption};
use config::{region_name, require_region, Preferences, DEFAULT_INSTANCE_TYPE, REGIONS};
use state::ProxyState;
use tokio::task::JoinSet;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(false)
        .without_time()
        .init();

    match cli.command {
        Commands::Start {
            region,
            port,
            instance_type,
            no_system_proxy,
        } => cmd_start(region, port, instance_type, no_system_proxy).await,
        Commands::Stop { force } => cmd_stop(force).await,
        Commands::Status => cmd_status(),
        Commands::ListRegions { detailed } => {
            cmd_list_regions(detailed);
            Ok(())
        }
        Commands::Cleanup { region } => cmd_cleanup(region.as_deref()).await,
        Commands::Config { action } => cmd_config(action),
    }
}

async fn cmd_start(
    region: Option<String>,
    port: Option<u16>,
    instance_type: Option<String>,
    no_system_proxy: bool,
) -> Result<()> {
    let prefs = Preferences::load()?;

    let region = match (region, prefs.default_region) {
        (Some(r), _) => r,
        (None, Some(r)) => {
            info!("Using default region from config: {}", r);
            r
        }
        (None, None) => bail!(
            "No region specified. Use --region or set a default with:\n  region-proxy config set-region <REGION>\n\nUse 'region-proxy list-regions' to see available regions."
        ),
    };

    let port = port.or(prefs.default_port).unwrap_or(1080);
    let instance_type = instance_type
        .or(prefs.default_instance_type)
        .unwrap_or_else(|| DEFAULT_INSTANCE_TYPE.to_string());
    let enable_system_proxy = !no_system_proxy && !prefs.no_system_proxy.unwrap_or(false);

    if ProxyState::load()?.is_some() {
        bail!("A proxy is already running. Use 'region-proxy stop' first.");
    }

    let region_info = require_region(&region)?;

    info!("🚀 Starting proxy in {} ({})", region_info.name, region);
    info!("   Instance type: {}", instance_type);
    info!("   Local port: {}", port);

    let ec2 = aws::Ec2Manager::new(&aws::load_config(&region).await, &region);

    info!("📦 Finding latest Amazon Linux 2023 AMI...");
    let ami_id = ec2
        .find_latest_ami(aws::is_arm_instance_type(&instance_type))
        .await?;

    info!("🔒 Creating security group...");
    let sg_id = ec2.create_security_group().await?;

    info!("🔑 Creating key pair...");
    let (key_name, private_key) = ec2.create_key_pair().await?;
    let key_path = ProxyState::write_private_key(&key_name, &private_key)?;

    info!("🖥️  Launching EC2 instance...");
    let instance_id = match ec2
        .launch_instance(&ami_id, &instance_type, &sg_id, &key_name)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            warn!("Cleaning up resources...");
            tolerate(true, ec2.delete_security_group(&sg_id).await)?;
            tolerate(true, ec2.delete_key_pair(&key_name).await)?;
            tolerate(true, store::remove_file_if_exists(&key_path).map(drop))?;
            return Err(e);
        }
    };

    let mut state = ProxyState {
        instance_id,
        region,
        public_ip: String::new(),
        security_group_id: sg_id,
        key_pair_name: key_name,
        key_path,
        local_port: port,
        system_proxy_service: None,
        started_at: Utc::now(),
    };
    state.save()?;

    if let Err(e) = connect(&ec2, &mut state, enable_system_proxy).await {
        error!("Failed to establish proxy: {:#}", e);
        warn!("Cleaning up resources...");
        release_resources(&ec2, &state, true).await?;
        return Err(e);
    }

    println!();
    println!("✅ Proxy is ready!");
    println!();
    println!("   Region:    {} ({})", region_info.name, state.region);
    println!("   Public IP: {}", state.public_ip);
    println!("   SOCKS:     localhost:{}", port);
    println!();
    println!("   To stop:   region-proxy stop");
    println!();

    Ok(())
}

/// Bring up the tunnel for an already launched instance, persisting progress to state
async fn connect(
    ec2: &aws::Ec2Manager,
    state: &mut ProxyState,
    enable_system_proxy: bool,
) -> Result<()> {
    info!("⏳ Waiting for instance to be ready...");
    state.public_ip = ec2.wait_for_instance(&state.instance_id).await?;
    state.save()?;

    info!("   Waiting for SSH port to open...");
    proxy::wait_for_port(&state.public_ip, proxy::SSH_PORT).await?;

    info!("🔗 Starting SSH tunnel...");
    proxy::start_ssh_tunnel(&state.public_ip, &state.key_path, state.local_port)?;

    if enable_system_proxy {
        info!("🌐 Configuring system proxy...");
        state.system_proxy_service = Some(proxy::enable_socks_proxy(state.local_port)?);
        state.save()?;
    }

    Ok(())
}

/// Tear down everything recorded in `state`. With `force`, failures are logged and skipped.
async fn release_resources(ec2: &aws::Ec2Manager, state: &ProxyState, force: bool) -> Result<()> {
    if let Some(service) = &state.system_proxy_service {
        info!("🌐 Disabling system proxy...");
        tolerate(force, proxy::disable_socks_proxy(service))?;
    }

    info!("🔗 Stopping SSH tunnel...");
    tolerate(force, proxy::stop_ssh_tunnel(state.local_port))?;

    info!("🖥️  Terminating EC2 instance...");
    tolerate(
        force,
        ec2.terminate_instances(std::slice::from_ref(&state.instance_id))
            .await,
    )?;

    info!("🔒 Deleting security group...");
    tolerate(
        force,
        ec2.delete_security_group(&state.security_group_id).await,
    )?;

    info!("🔑 Deleting key pair...");
    tolerate(force, ec2.delete_key_pair(&state.key_pair_name).await)?;

    tolerate(
        true,
        store::remove_file_if_exists(&state.key_path).map(drop),
    )?;
    ProxyState::delete()
}

fn tolerate(force: bool, result: Result<()>) -> Result<()> {
    match result {
        Err(e) if force => {
            warn!("{:#}", e);
            Ok(())
        }
        other => other,
    }
}

async fn cmd_stop(force: bool) -> Result<()> {
    let Some(state) = ProxyState::load()? else {
        if force {
            warn!("No active proxy found, but --force was specified. Skipping.");
            return Ok(());
        }
        bail!("No active proxy found. Nothing to stop.");
    };

    info!("🛑 Stopping proxy...");
    let ec2 = aws::Ec2Manager::new(&aws::load_config(&state.region).await, &state.region);
    release_resources(&ec2, &state, force).await?;

    println!();
    println!("✅ Proxy stopped and cleaned up!");
    println!();

    Ok(())
}

fn cmd_status() -> Result<()> {
    let Some(state) = ProxyState::load()? else {
        println!("No active proxy.");
        return Ok(());
    };

    let duration = Utc::now().signed_duration_since(state.started_at);
    let ssh_running = proxy::find_ssh_pid(state.local_port)?.is_some();
    let proxy_enabled = state
        .system_proxy_service
        .as_deref()
        .is_some_and(|s| proxy::is_socks_proxy_enabled(s).unwrap_or(false));

    println!();
    println!("📊 Proxy Status");
    println!();
    println!(
        "   Region:      {} ({})",
        region_name(&state.region),
        state.region
    );
    println!("   Instance:    {}", state.instance_id);
    println!("   Public IP:   {}", state.public_ip);
    println!("   SOCKS:       localhost:{}", state.local_port);
    println!(
        "   SSH tunnel:  {}",
        if ssh_running {
            "✅ Running"
        } else {
            "❌ Not running"
        }
    );
    println!(
        "   System proxy: {}",
        if proxy_enabled {
            "✅ Enabled"
        } else {
            "❌ Disabled"
        }
    );
    println!(
        "   Running for: {}h {}m",
        duration.num_hours(),
        duration.num_minutes() % 60
    );
    println!();

    Ok(())
}

fn cmd_list_regions(detailed: bool) {
    println!();
    println!("Available AWS Regions:");
    println!();

    if detailed {
        println!("{:<20} Name", "Code");
        println!("{}", "-".repeat(40));
        for region in REGIONS {
            println!("{:<20} {}", region.code, region.name);
        }
    } else {
        for region in REGIONS {
            println!("  {} ({})", region.code, region.name);
        }
    }
    println!();
}

async fn cmd_cleanup(region: Option<&str>) -> Result<()> {
    let regions: Vec<&'static str> = match region {
        Some(r) => vec![require_region(r)?.code],
        None => REGIONS.iter().map(|r| r.code).collect(),
    };

    let config = aws::load_config(regions[0]).await;
    let mut set = JoinSet::new();
    for region_code in regions {
        let ec2 = aws::Ec2Manager::new(&config, region_code);
        set.spawn(cleanup_region(ec2, region_code));
    }

    let mut total_cleaned = 0u32;
    while let Some(res) = set.join_next().await {
        match res {
            Ok(Ok(n)) => total_cleaned += n,
            Ok(Err(e)) => warn!("Region cleanup failed: {:#}", e),
            Err(e) => warn!("Task join error: {}", e),
        }
    }

    if total_cleaned == 0 {
        println!("No orphaned resources found.");
    } else {
        println!();
        println!("Cleaned up {} resource(s).", total_cleaned);
    }

    Ok(())
}

async fn cleanup_region(ec2: aws::Ec2Manager, region_code: &'static str) -> Result<u32> {
    info!("Checking region: {}", region_code);
    let orphaned = ec2.find_orphaned_resources().await?;
    if orphaned.is_empty() {
        return Ok(0);
    }

    println!("Found orphaned resources in {}:", region_code);
    let mut cleaned = 0u32;

    if !orphaned.instance_ids.is_empty() {
        println!(
            "  Terminating instance(s): {}",
            orphaned.instance_ids.join(", ")
        );
        match ec2.terminate_instances(&orphaned.instance_ids).await {
            Ok(()) => cleaned += orphaned.instance_ids.len() as u32,
            Err(e) => warn!("Failed to terminate instances in {}: {:#}", region_code, e),
        }
    }

    for id in &orphaned.security_group_ids {
        println!("  Deleting security group: {}", id);
        match ec2.delete_security_group(id).await {
            Ok(()) => cleaned += 1,
            Err(e) => warn!("Failed to delete security group {}: {:#}", id, e),
        }
    }

    for name in &orphaned.key_pair_names {
        println!("  Deleting key pair: {}", name);
        match ec2.delete_key_pair(name).await {
            Ok(()) => cleaned += 1,
            Err(e) => warn!("Failed to delete key pair {}: {:#}", name, e),
        }
    }

    Ok(cleaned)
}

fn cmd_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            let prefs = Preferences::load()?;
            println!();
            println!("⚙️  Configuration");
            println!();

            if prefs.is_empty() {
                println!("   No configuration set.");
                println!();
                println!("   Set defaults with:");
                println!("     region-proxy config set-region <REGION>");
                println!("     region-proxy config set-port <PORT>");
            } else {
                if let Some(region) = &prefs.default_region {
                    println!(
                        "   Default region:        {} ({})",
                        region,
                        region_name(region)
                    );
                }
                if let Some(port) = prefs.default_port {
                    println!("   Default port:          {}", port);
                }
                if let Some(instance_type) = &prefs.default_instance_type {
                    println!("   Default instance type: {}", instance_type);
                }
                if let Some(no_system_proxy) = prefs.no_system_proxy {
                    println!("   Skip system proxy:     {}", no_system_proxy);
                }
            }

            println!();
            println!(
                "   Config file: {}",
                Preferences::config_file_path()?.display()
            );
            println!();
        }

        ConfigAction::SetRegion { region } => {
            let name = require_region(&region)?.name;
            Preferences::update(|p| p.default_region = Some(region.clone()))?;
            println!("✅ Default region set to: {} ({})", region, name);
        }

        ConfigAction::SetPort { port } => {
            if port == 0 {
                bail!("Port must be greater than 0");
            }
            Preferences::update(|p| p.default_port = Some(port))?;
            println!("✅ Default port set to: {}", port);
        }

        ConfigAction::SetInstanceType { instance_type } => {
            Preferences::update(|p| p.default_instance_type = Some(instance_type.clone()))?;
            println!("✅ Default instance type set to: {}", instance_type);
        }

        ConfigAction::SetNoSystemProxy { value } => {
            Preferences::update(|p| p.no_system_proxy = Some(value))?;
            if value {
                println!("✅ System proxy configuration will be skipped by default");
            } else {
                println!("✅ System proxy will be configured by default");
            }
        }

        ConfigAction::Unset { option } => {
            Preferences::update(|p| match option {
                UnsetOption::Region => p.default_region = None,
                UnsetOption::Port => p.default_port = None,
                UnsetOption::InstanceType => p.default_instance_type = None,
                UnsetOption::NoSystemProxy => p.no_system_proxy = None,
            })?;
            let name = option.to_possible_value().map(|v| v.get_name().to_string());
            println!("✅ Cleared '{}'", name.unwrap_or_default());
        }

        ConfigAction::Reset => {
            if Preferences::delete()? {
                println!("✅ Configuration reset to defaults");
            } else {
                println!("No configuration file to reset.");
            }
        }
    }

    Ok(())
}
