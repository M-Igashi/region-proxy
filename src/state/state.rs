use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Represents the current state of the proxy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyState {
    pub instance_id: String,
    pub region: String,
    pub public_ip: String,
    pub security_group_id: String,
    pub key_pair_name: String,
    pub key_path: PathBuf,
    pub local_port: u16,
    pub ssh_pid: Option<u32>,
    pub started_at: DateTime<Utc>,
}

impl ProxyState {
    /// Get the state file path
    pub fn state_file_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let state_dir = home.join(".region-proxy");
        fs::create_dir_all(&state_dir)?;
        Ok(state_dir.join("state.json"))
    }

    /// Get the keys directory path
    pub fn keys_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let keys_dir = home.join(".region-proxy").join("keys");
        fs::create_dir_all(&keys_dir)?;
        Ok(keys_dir)
    }

    /// Load the current state from file
    pub fn load() -> Result<Option<Self>> {
        let path = Self::state_file_path()?;
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        let state: Self = serde_json::from_str(&content)?;
        Ok(Some(state))
    }

    /// Save the current state to file
    pub fn save(&self) -> Result<()> {
        let path = Self::state_file_path()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Delete the state file
    pub fn delete() -> Result<()> {
        let path = Self::state_file_path()?;
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Check if a proxy is currently running
    pub fn is_running() -> Result<bool> {
        Ok(Self::load()?.is_some())
    }
}
