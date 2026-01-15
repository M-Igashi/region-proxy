use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
    pub fn state_file_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let state_dir = home.join(".region-proxy");
        fs::create_dir_all(&state_dir)?;
        Ok(state_dir.join("state.json"))
    }

    pub fn keys_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let keys_dir = home.join(".region-proxy").join("keys");
        fs::create_dir_all(&keys_dir)?;
        Ok(keys_dir)
    }

    pub fn load() -> Result<Option<Self>> {
        let path = Self::state_file_path()?;
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&path)?;
        let state: Self = serde_json::from_str(&content)?;
        Ok(Some(state))
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::state_file_path()?;
        let content = serde_json::to_string(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    pub fn delete() -> Result<()> {
        let path = Self::state_file_path()?;
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    pub fn is_running() -> Result<bool> {
        Ok(Self::load()?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn create_test_state() -> ProxyState {
        ProxyState {
            instance_id: "i-1234567890abcdef0".to_string(),
            region: "ap-northeast-1".to_string(),
            public_ip: "54.150.123.45".to_string(),
            security_group_id: "sg-0123456789abcdef0".to_string(),
            key_pair_name: "region-proxy-test-key".to_string(),
            key_path: PathBuf::from("/tmp/test-key.pem"),
            local_port: 1080,
            ssh_pid: Some(12345),
            started_at: Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap(),
        }
    }

    #[test]
    fn test_serialize_deserialize() {
        let state = create_test_state();
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: ProxyState = serde_json::from_str(&json).unwrap();

        assert_eq!(state.instance_id, deserialized.instance_id);
        assert_eq!(state.region, deserialized.region);
        assert_eq!(state.public_ip, deserialized.public_ip);
        assert_eq!(state.security_group_id, deserialized.security_group_id);
        assert_eq!(state.key_pair_name, deserialized.key_pair_name);
        assert_eq!(state.key_path, deserialized.key_path);
        assert_eq!(state.local_port, deserialized.local_port);
        assert_eq!(state.ssh_pid, deserialized.ssh_pid);
        assert_eq!(state.started_at, deserialized.started_at);
    }

    #[test]
    fn test_serialize_without_ssh_pid() {
        let mut state = create_test_state();
        state.ssh_pid = None;

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: ProxyState = serde_json::from_str(&json).unwrap();

        assert!(deserialized.ssh_pid.is_none());
    }

    #[test]
    fn test_json_format() {
        let state = create_test_state();
        let json = serde_json::to_string_pretty(&state).unwrap();

        assert!(json.contains("instance_id"));
        assert!(json.contains("i-1234567890abcdef0"));
        assert!(json.contains("region"));
        assert!(json.contains("ap-northeast-1"));
    }

    #[test]
    fn test_state_file_path() {
        let path = ProxyState::state_file_path().unwrap();
        assert!(path.to_string_lossy().contains(".region-proxy"));
        assert!(path.to_string_lossy().ends_with("state.json"));
    }

    #[test]
    fn test_keys_dir() {
        let path = ProxyState::keys_dir().unwrap();
        assert!(path.to_string_lossy().contains(".region-proxy"));
        assert!(path.to_string_lossy().ends_with("keys"));
    }
}
