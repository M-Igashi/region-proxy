use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info};

/// Start SSH dynamic forwarding in the background
pub fn start_ssh_tunnel(host: &str, key_path: &Path, local_port: u16, user: &str) -> Result<u32> {
    info!(
        "Starting SSH tunnel to {}@{} on port {}",
        user, host, local_port
    );

    // Set correct permissions on key file
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(key_path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(key_path, perms)?;
    }

    let child = Command::new("ssh")
        .arg("-f") // Background
        .arg("-N") // No command
        .arg("-D")
        .arg(format!("{}", local_port))
        .arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("UserKnownHostsFile=/dev/null")
        .arg("-o")
        .arg("ServerAliveInterval=60")
        .arg("-o")
        .arg("ServerAliveCountMax=3")
        .arg("-i")
        .arg(key_path)
        .arg(format!("{}@{}", user, host))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to start SSH process")?;

    let pid = child.id();
    info!("SSH tunnel started with PID: {}", pid);

    Ok(pid)
}

/// Find the SSH process by port
pub fn find_ssh_pid(port: u16) -> Result<Option<u32>> {
    let output = Command::new("lsof")
        .arg("-i")
        .arg(format!(":{}", port))
        .arg("-t")
        .output()
        .context("Failed to run lsof")?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Ok(pid) = line.trim().parse::<u32>() {
            return Ok(Some(pid));
        }
    }

    Ok(None)
}

/// Stop the SSH tunnel by PID
pub fn stop_ssh_tunnel(pid: u32) -> Result<()> {
    info!("Stopping SSH tunnel (PID: {})", pid);

    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
            .context("Failed to send SIGTERM to SSH process")?;
    }

    #[cfg(not(unix))]
    {
        Command::new("kill")
            .arg(pid.to_string())
            .status()
            .context("Failed to kill SSH process")?;
    }

    info!("SSH tunnel stopped");
    Ok(())
}

/// Stop SSH tunnel by port
pub fn stop_ssh_tunnel_by_port(port: u16) -> Result<()> {
    if let Some(pid) = find_ssh_pid(port)? {
        stop_ssh_tunnel(pid)?;
    } else {
        debug!("No SSH process found on port {}", port);
    }
    Ok(())
}

/// Wait for SSH tunnel to be ready
pub async fn wait_for_tunnel(port: u16) -> Result<()> {
    info!("Waiting for SSH tunnel to be ready...");

    for attempt in 1..=30 {
        let output = Command::new("nc")
            .arg("-z")
            .arg("localhost")
            .arg(port.to_string())
            .output();

        match output {
            Ok(o) if o.status.success() => {
                info!("SSH tunnel is ready");
                return Ok(());
            }
            _ => {
                debug!("Tunnel not ready yet (attempt {}/30)", attempt);
                sleep(Duration::from_secs(1)).await;
            }
        }
    }

    bail!("Timeout waiting for SSH tunnel to be ready");
}
