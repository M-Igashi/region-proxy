use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use tracing::debug;

const SSH_USER: &str = "ec2-user";

/// Start SSH dynamic forwarding in the background.
/// ssh retries the connection while the instance boots (`ConnectionAttempts`), and
/// `-f` returns once authentication and the local forward succeed, so a successful
/// exit status means the tunnel is up.
pub fn start_ssh_tunnel(host: &str, key_path: &Path, local_port: u16) -> Result<()> {
    debug!(
        "Starting SSH tunnel to {}@{} on port {}",
        SSH_USER, host, local_port
    );

    let status = Command::new("ssh")
        .args(["-f", "-N", "-D"])
        .arg(local_port.to_string())
        .args([
            "-o",
            "ConnectTimeout=5",
            "-o",
            "ConnectionAttempts=60",
            "-o",
            "ExitOnForwardFailure=yes",
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "UserKnownHostsFile=/dev/null",
            "-o",
            "ServerAliveInterval=60",
            "-o",
            "ServerAliveCountMax=3",
        ])
        .arg("-i")
        .arg(key_path)
        .arg(format!("{}@{}", SSH_USER, host))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("Failed to start SSH process")?;

    if !status.success() {
        bail!("SSH failed to establish tunnel ({})", status);
    }

    debug!("SSH tunnel is ready");
    Ok(())
}

/// Find the ssh process listening on the given local port
pub fn find_ssh_pid(port: u16) -> Result<Option<u32>> {
    let output = Command::new("lsof")
        .args(["-a", "-c", "ssh", "-nP", "-t", "-sTCP:LISTEN"])
        .arg(format!("-iTCP:{}", port))
        .output()
        .context("Failed to run lsof")?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.trim().parse().ok()))
}

/// Stop the SSH tunnel listening on the given local port
pub fn stop_ssh_tunnel(port: u16) -> Result<()> {
    let Some(pid) = find_ssh_pid(port)? else {
        debug!("No SSH process found on port {}", port);
        return Ok(());
    };

    debug!("Stopping SSH tunnel (PID: {})", pid);
    let status = Command::new("kill")
        .arg(pid.to_string())
        .status()
        .context("Failed to run kill")?;
    if !status.success() {
        bail!("Failed to stop SSH process {}", pid);
    }
    Ok(())
}
