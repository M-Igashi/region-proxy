use anyhow::{bail, Context, Result};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};
use tracing::{debug, info};

pub const SSH_PORT: u16 = 22;
const SSH_USER: &str = "ec2-user";

/// Start SSH dynamic forwarding in the background.
/// `ssh -f` returns once authentication and the local forward succeed, so a
/// successful exit status means the tunnel is up.
pub fn start_ssh_tunnel(host: &str, key_path: &Path, local_port: u16) -> Result<()> {
    info!(
        "Starting SSH tunnel to {}@{} on port {}",
        SSH_USER, host, local_port
    );

    let status = Command::new("ssh")
        .args(["-f", "-N", "-D"])
        .arg(local_port.to_string())
        .args([
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

    info!("SSH tunnel is ready");
    Ok(())
}

/// Find the process listening on the given local port
pub fn find_ssh_pid(port: u16) -> Result<Option<u32>> {
    let output = Command::new("lsof")
        .args(["-nP", "-t", "-sTCP:LISTEN"])
        .arg(format!("-iTCP:{}", port))
        .output()
        .context("Failed to run lsof")?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.trim().parse().ok()))
}

/// Stop the SSH tunnel listening on the given local port
pub fn stop_ssh_tunnel(port: u16) -> Result<()> {
    match find_ssh_pid(port)? {
        Some(pid) => {
            info!("Stopping SSH tunnel (PID: {})", pid);
            kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
                .context("Failed to send SIGTERM to SSH process")?;
        }
        None => debug!("No SSH process found on port {}", port),
    }
    Ok(())
}

/// Wait until a TCP port on the given host accepts connections
pub async fn wait_for_port(host: &str, port: u16) -> Result<()> {
    const MAX_ATTEMPTS: u32 = 60;
    for attempt in 1..=MAX_ATTEMPTS {
        if timeout(Duration::from_secs(2), TcpStream::connect((host, port)))
            .await
            .is_ok_and(|r| r.is_ok())
        {
            debug!("Port {} open on {} (attempt {})", port, host, attempt);
            return Ok(());
        }
        if attempt < MAX_ATTEMPTS {
            sleep(Duration::from_millis(500)).await;
        }
    }
    bail!("Timeout waiting for port {} on {}", port, host);
}
