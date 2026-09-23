use anyhow::{bail, Context, Result};
use std::process::Command;
use tracing::{debug, info, warn};

fn networksetup(args: &[&str]) -> Result<String> {
    let output = Command::new("networksetup")
        .args(args)
        .output()
        .context("Failed to run networksetup")?;

    if !output.status.success() {
        bail!(
            "networksetup {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn active_network_service() -> Result<String> {
    let services = networksetup(&["-listallnetworkservices"])?;

    for service in [
        "Wi-Fi",
        "Ethernet",
        "USB 10/100/1000 LAN",
        "Thunderbolt Ethernet",
    ] {
        if !services.lines().any(|line| line == service) {
            continue;
        }
        let Ok(info) = networksetup(&["-getinfo", service]) else {
            continue;
        };
        let has_ip = info
            .lines()
            .any(|line| line.starts_with("IP address:") && !line.ends_with("none"));
        if has_ip {
            debug!("Found active network service: {}", service);
            return Ok(service.to_string());
        }
    }

    warn!("Could not detect active network service, falling back to Wi-Fi");
    Ok("Wi-Fi".to_string())
}

/// Enable the SOCKS proxy on the active network service and return its name
pub fn enable_socks_proxy(port: u16) -> Result<String> {
    let service = active_network_service()?;
    info!("Enabling SOCKS proxy on {} (localhost:{})", service, port);

    let port = port.to_string();
    networksetup(&["-setsocksfirewallproxy", &service, "localhost", &port])?;
    networksetup(&["-setsocksfirewallproxystate", &service, "on"])?;

    info!("SOCKS proxy enabled");
    Ok(service)
}

pub fn disable_socks_proxy(service: &str) -> Result<()> {
    info!("Disabling SOCKS proxy on {}", service);
    networksetup(&["-setsocksfirewallproxystate", service, "off"])?;
    info!("SOCKS proxy disabled");
    Ok(())
}

pub fn is_socks_proxy_enabled(service: &str) -> Result<bool> {
    Ok(networksetup(&["-getsocksfirewallproxy", service])?.contains("Enabled: Yes"))
}
