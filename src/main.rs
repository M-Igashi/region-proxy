mod aws;
mod cli;
mod config;
mod proxy;
mod state;

use anyhow::{bail, Context, Result};
use chrono::Utc;
use clap::Parser;
use cli::{Cli, Commands, ConfigAction};
use config::{find_region, Preferences, REGIONS};
use state::ProxyState;
use std::fs;
use tracing::{error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
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
        } => {
            cmd_start(region, port, instance_type, no_system_proxy).await?;
        }
        Commands::Stop { force } => {
            cmd_stop(force).await?;
        }
        Commands::Status => {
            cmd_status().await?;
        }
        Commands::ListRegions { detailed } => {
            cmd_list_regions(detailed);
        }
        Commands::Cleanup { region } => {
            cmd_cleanup(region.as_deref()).await?;
        }
        Commands::Config { action } => {
            cmd_config(action)?;
        }
    }

    Ok(())
}

async fn cmd_start(
    region: Option<String>,
    port: Option<u16>,
    instance_type: Option<String>,
    no_system_proxy: bool,
) -> Result<()> {
    // Load preferences
    let prefs = Preferences::load()?;

    // Resolve region from CLI arg or preferences
    let region = match region {
        Some(r) => r,
        None => match prefs.default_region {
            Some(r) => {
                info!("Using default region from config: {}", r);
                r
            }
            None => {
                bail!(
                    "No region specified. Use --region or set a default with:\n  region-proxy config set-region <REGION>\n\nUse 'region-proxy list-regions' to see available regions."
                );
            }
        },
    };

    // Resolve port from CLI arg or preferences
    let port = port.or(prefs.default_port).unwrap_or(1080);

    // Resolve instance_type from CLI arg or preferences
    let instance_type = instance_type.or(prefs.default_instance_type);

    // Resolve no_system_proxy from CLI flag or preferences
    let enable_system_proxy = !no_system_proxy && !prefs.no_system_proxy.unwrap_or(false);

    // Check if already running
    if ProxyState::is_running()? {
        bail!("A proxy is already running. Use 'region-proxy stop' first.");
    }

    // Validate region
    let region_info = find_region(&region).with_context(|| {
        format!(
            "Unknown region: {}. Use 'region-proxy list-regions' to see available regions.",
            region
        )
    })?;

    let instance_type = instance_type
        .as_deref()
        .unwrap_or(region_info.default_instance_type());
    let is_arm = instance_type.starts_with("t4g")
        || instance_type.starts_with("m7g")
        || instance_type.starts_with("c7g");

    info!("🚀 Starting proxy in {} ({})", region_info.name, region);
    info!("   Instance type: {}", instance_type);
    info!("   Local port: {}", port);

    // Create EC2 manager
    let ec2 = aws::Ec2Manager::new(&region).await?;

    // Find AMI
    info!("📦 Finding latest Amazon Linux 2023 AMI...");
    let ami_id = ec2.find_latest_ami(is_arm).await?;

    // Create security group
    info!("🔒 Creating security group...");
    let sg_id = ec2.create_security_group().await?;

    // Create key pair
    info!("🔑 Creating key pair...");
    let (key_name, private_key) = ec2.create_key_pair().await?;

    // Save key to file
    let keys_dir = ProxyState::keys_dir()?;
    let key_path = keys_dir.join(format!("{}.pem", key_name));
    fs::write(&key_path, &private_key)?;

    // Launch instance
    info!("🖥️  Launching EC2 instance...");
    let instance_id = ec2
        .launch_instance(&ami_id, instance_type, &sg_id, &key_name)
        .await?;

    // Wait for instance
    info!("⏳ Waiting for instance to be ready...");
    let public_ip = match ec2.wait_for_instance(&instance_id).await {
        Ok(ip) => ip,
        Err(e) => {
            // Cleanup on failure
            error!("Failed to wait for instance: {}", e);
            warn!("Cleaning up resources...");
            let _ = ec2.terminate_instance(&instance_id).await;
            let _ = ec2.delete_security_group(&sg_id).await;
            let _ = ec2.delete_key_pair(&key_name).await;
            let _ = fs::remove_file(&key_path);
            return Err(e);
        }
    };

    // Start SSH tunnel
    info!("🔗 Starting SSH tunnel...");
    let ssh_pid = proxy::start_ssh_tunnel(&public_ip, &key_path, port, "ec2-user")?;

    // Wait for tunnel
    proxy::wait_for_tunnel(port).await?;

    // Enable system proxy
    if enable_system_proxy {
        info!("🌐 Configuring system proxy...");
        proxy::enable_socks_proxy(port)?;
    }

    // Save state
    let state = ProxyState {
        instance_id: instance_id.clone(),
        region: region.to_string(),
        public_ip: public_ip.clone(),
        security_group_id: sg_id,
        key_pair_name: key_name,
        key_path,
        local_port: port,
        ssh_pid: Some(ssh_pid),
        started_at: Utc::now(),
    };
    state.save()?;

    println!();
    println!("✅ Proxy is ready!");
    println!();
    println!("   Region:    {} ({})", region_info.name, region);
    println!("   Public IP: {}", public_ip);
    println!("   SOCKS:     localhost:{}", port);
    println!();
    println!("   To stop:   region-proxy stop");
    println!();

    Ok(())
}

async fn cmd_stop(force: bool) -> Result<()> {
    let state = match ProxyState::load()? {
        Some(s) => s,
        None => {
            if force {
                warn!("No active proxy found, but --force was specified. Skipping.");
                return Ok(());
            }
            bail!("No active proxy found. Nothing to stop.");
        }
    };

    info!("🛑 Stopping proxy...");

    // Disable system proxy
    info!("🌐 Disabling system proxy...");
    if let Err(e) = proxy::disable_socks_proxy() {
        if force {
            warn!("Failed to disable system proxy: {}", e);
        } else {
            return Err(e);
        }
    }

    // Stop SSH tunnel
    info!("🔗 Stopping SSH tunnel...");
    if let Some(pid) = state.ssh_pid {
        if let Err(e) = proxy::stop_ssh_tunnel(pid) {
            if force {
                warn!("Failed to stop SSH tunnel: {}", e);
            } else {
                // Try by port
                let _ = proxy::stop_ssh_tunnel_by_port(state.local_port);
            }
        }
    } else {
        let _ = proxy::stop_ssh_tunnel_by_port(state.local_port);
    }

    // Terminate EC2 instance
    info!("🖥️  Terminating EC2 instance...");
    let ec2 = aws::Ec2Manager::new(&state.region).await?;
    if let Err(e) = ec2.terminate_instance(&state.instance_id).await {
        if force {
            warn!("Failed to terminate instance: {}", e);
        } else {
            return Err(e);
        }
    }

    // Delete security group
    info!("🔒 Deleting security group...");
    if let Err(e) = ec2.delete_security_group(&state.security_group_id).await {
        if force {
            warn!("Failed to delete security group: {}", e);
        } else {
            return Err(e);
        }
    }

    // Delete key pair
    info!("🔑 Deleting key pair...");
    if let Err(e) = ec2.delete_key_pair(&state.key_pair_name).await {
        if force {
            warn!("Failed to delete key pair: {}", e);
        } else {
            return Err(e);
        }
    }

    // Delete local key file
    if state.key_path.exists() {
        let _ = fs::remove_file(&state.key_path);
    }

    // Delete state file
    ProxyState::delete()?;

    println!();
    println!("✅ Proxy stopped and cleaned up!");
    println!();

    Ok(())
}

async fn cmd_status() -> Result<()> {
    let state = match ProxyState::load()? {
        Some(s) => s,
        None => {
            println!("No active proxy.");
            return Ok(());
        }
    };

    let region_info = find_region(&state.region);
    let region_name = region_info.map(|r| r.name).unwrap_or("Unknown");

    let duration = Utc::now().signed_duration_since(state.started_at);
    let hours = duration.num_hours();
    let minutes = duration.num_minutes() % 60;

    let ssh_running = proxy::find_ssh_pid(state.local_port)?.is_some();
    let proxy_enabled = proxy::is_socks_proxy_enabled().unwrap_or(false);

    println!();
    println!("📊 Proxy Status");
    println!();
    println!("   Region:      {} ({})", region_name, state.region);
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
    println!("   Running for: {}h {}m", hours, minutes);
    println!();

    Ok(())
}

fn cmd_list_regions(detailed: bool) {
    println!();
    println!("Available AWS Regions:");
    println!();

    if detailed {
        println!("{:<20} {:<20} Default Instance", "Code", "Name");
        println!("{}", "-".repeat(55));
        for region in REGIONS {
            println!(
                "{:<20} {:<20} {}",
                region.code,
                region.name,
                region.default_instance_type()
            );
        }
    } else {
        for region in REGIONS {
            println!("  {} ({})", region.code, region.name);
        }
    }
    println!();
}

async fn cmd_cleanup(region: Option<&str>) -> Result<()> {
    let regions: Vec<&str> = match region {
        Some(r) => vec![r],
        None => REGIONS.iter().map(|r| r.code).collect(),
    };

    let mut total_cleaned = 0;

    for region_code in regions {
        info!("Checking region: {}", region_code);
        let ec2 = aws::Ec2Manager::new(region_code).await?;
        let orphaned = ec2.find_orphaned_resources().await?;

        if orphaned.is_empty() {
            continue;
        }

        println!("Found orphaned resources in {}:", region_code);

        for id in &orphaned.instance_ids {
            println!("  Terminating instance: {}", id);
            if let Err(e) = ec2.terminate_instance(id).await {
                warn!("Failed to terminate instance {}: {}", id, e);
            } else {
                total_cleaned += 1;
            }
        }

        for id in &orphaned.security_group_ids {
            println!("  Deleting security group: {}", id);
            if let Err(e) = ec2.delete_security_group(id).await {
                warn!("Failed to delete security group {}: {}", id, e);
            } else {
                total_cleaned += 1;
            }
        }

        for name in &orphaned.key_pair_names {
            println!("  Deleting key pair: {}", name);
            if let Err(e) = ec2.delete_key_pair(name).await {
                warn!("Failed to delete key pair {}: {}", name, e);
            } else {
                total_cleaned += 1;
            }
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
                if let Some(ref region) = prefs.default_region {
                    let region_name = find_region(region).map(|r| r.name).unwrap_or("Unknown");
                    println!("   Default region:        {} ({})", region, region_name);
                }
                if let Some(port) = prefs.default_port {
                    println!("   Default port:          {}", port);
                }
                if let Some(ref instance_type) = prefs.default_instance_type {
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
            // Validate region
            if find_region(&region).is_none() {
                bail!(
                    "Unknown region: {}. Use 'region-proxy list-regions' to see available regions.",
                    region
                );
            }

            let mut prefs = Preferences::load()?;
            prefs.set_default_region(Some(region.clone()));
            prefs.save()?;

            let region_name = find_region(&region).map(|r| r.name).unwrap_or("Unknown");
            println!("✅ Default region set to: {} ({})", region, region_name);
        }

        ConfigAction::SetPort { port } => {
            if port == 0 {
                bail!("Port must be greater than 0");
            }

            let mut prefs = Preferences::load()?;
            prefs.set_default_port(Some(port));
            prefs.save()?;

            println!("✅ Default port set to: {}", port);
        }

        ConfigAction::SetInstanceType { instance_type } => {
            let mut prefs = Preferences::load()?;
            prefs.set_default_instance_type(Some(instance_type.clone()));
            prefs.save()?;

            println!("✅ Default instance type set to: {}", instance_type);
        }

        ConfigAction::SetNoSystemProxy { value } => {
            let value = match value.to_lowercase().as_str() {
                "true" | "1" | "yes" => true,
                "false" | "0" | "no" => false,
                _ => bail!("Invalid value: {}. Use 'true' or 'false'", value),
            };

            let mut prefs = Preferences::load()?;
            prefs.set_no_system_proxy(Some(value));
            prefs.save()?;

            if value {
                println!("✅ System proxy configuration will be skipped by default");
            } else {
                println!("✅ System proxy will be configured by default");
            }
        }

        ConfigAction::Unset { option } => {
            let mut prefs = Preferences::load()?;

            match option.as_str() {
                "region" => {
                    prefs.set_default_region(None);
                    prefs.save()?;
                    println!("✅ Default region cleared");
                }
                "port" => {
                    prefs.set_default_port(None);
                    prefs.save()?;
                    println!("✅ Default port cleared");
                }
                "instance-type" => {
                    prefs.set_default_instance_type(None);
                    prefs.save()?;
                    println!("✅ Default instance type cleared");
                }
                "no-system-proxy" => {
                    prefs.set_no_system_proxy(None);
                    prefs.save()?;
                    println!("✅ System proxy preference cleared");
                }
                _ => {
                    bail!(
                        "Unknown option: {}. Valid options: region, port, instance-type, no-system-proxy",
                        option
                    );
                }
            }
        }

        ConfigAction::Reset => {
            let path = Preferences::config_file_path()?;
            if path.exists() {
                fs::remove_file(&path)?;
                println!("✅ Configuration reset to defaults");
            } else {
                println!("No configuration file to reset.");
            }
        }
    }

    Ok(())
}
