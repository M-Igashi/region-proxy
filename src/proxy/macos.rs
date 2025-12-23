use anyhow::{Context, Result};
use std::process::Command;
use tracing::{debug, info};

/// Get the active network service name (e.g., "Wi-Fi", "Ethernet")
pub fn get_active_network_service() -> Result<String> {
    // Get the list of network services
    let output = Command::new("networksetup")
        .arg("-listallnetworkservices")
        .output()
        .context("Failed to list network services")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Try common service names in order of preference
    let preferred_services = [
        "Wi-Fi",
        "Ethernet",
        "USB 10/100/1000 LAN",
        "Thunderbolt Ethernet",
    ];

    for service in preferred_services {
        if stdout.contains(service) {
            // Verify the service is active
            let status = Command::new("networksetup")
                .arg("-getinfo")
                .arg(service)
                .output();

            if let Ok(status_output) = status {
                let status_str = String::from_utf8_lossy(&status_output.stdout);
                if status_str.contains("IP address:") && !status_str.contains("IP address: none") {
                    debug!("Found active network service: {}", service);
                    return Ok(service.to_string());
                }
            }
        }
    }

    // Fallback to Wi-Fi
    Ok("Wi-Fi".to_string())
}

/// Enable SOCKS proxy on macOS
pub fn enable_socks_proxy(port: u16) -> Result<()> {
    let service = get_active_network_service()?;
    info!("Enabling SOCKS proxy on {} (localhost:{})", service, port);

    // Set SOCKS proxy server
    let status = Command::new("networksetup")
        .arg("-setsocksfirewallproxy")
        .arg(&service)
        .arg("localhost")
        .arg(port.to_string())
        .status()
        .context("Failed to set SOCKS proxy")?;

    if !status.success() {
        anyhow::bail!("networksetup command failed");
    }

    // Enable SOCKS proxy
    let status = Command::new("networksetup")
        .arg("-setsocksfirewallproxystate")
        .arg(&service)
        .arg("on")
        .status()
        .context("Failed to enable SOCKS proxy")?;

    if !status.success() {
        anyhow::bail!("Failed to enable SOCKS proxy state");
    }

    info!("SOCKS proxy enabled");
    Ok(())
}

/// Disable SOCKS proxy on macOS
pub fn disable_socks_proxy() -> Result<()> {
    let service = get_active_network_service()?;
    info!("Disabling SOCKS proxy on {}", service);

    let status = Command::new("networksetup")
        .arg("-setsocksfirewallproxystate")
        .arg(&service)
        .arg("off")
        .status()
        .context("Failed to disable SOCKS proxy")?;

    if !status.success() {
        anyhow::bail!("Failed to disable SOCKS proxy state");
    }

    info!("SOCKS proxy disabled");
    Ok(())
}

/// Check if SOCKS proxy is currently enabled
pub fn is_socks_proxy_enabled() -> Result<bool> {
    let service = get_active_network_service()?;

    let output = Command::new("networksetup")
        .arg("-getsocksfirewallproxy")
        .arg(&service)
        .output()
        .context("Failed to get SOCKS proxy status")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.contains("Enabled: Yes"))
}

/// Get current SOCKS proxy settings
#[allow(dead_code)]
pub fn get_socks_proxy_settings() -> Result<Option<(String, u16)>> {
    let service = get_active_network_service()?;

    let output = Command::new("networksetup")
        .arg("-getsocksfirewallproxy")
        .arg(&service)
        .output()
        .context("Failed to get SOCKS proxy settings")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if !stdout.contains("Enabled: Yes") {
        return Ok(None);
    }

    let mut server = None;
    let mut port = None;

    for line in stdout.lines() {
        if let Some(s) = line.strip_prefix("Server: ") {
            server = Some(s.trim().to_string());
        }
        if let Some(p) = line.strip_prefix("Port: ") {
            port = p.trim().parse().ok();
        }
    }

    match (server, port) {
        (Some(s), Some(p)) => Ok(Some((s, p))),
        _ => Ok(None),
    }
}
