use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

use crate::store::{app_dir, load_json, remove_file_if_exists, save_json};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProxyState {
    pub instance_id: String,
    pub region: String,
    pub public_ip: String,
    pub security_group_id: String,
    pub key_pair_name: String,
    pub key_path: PathBuf,
    pub local_port: u16,
    pub system_proxy_service: Option<String>,
    pub started_at: DateTime<Utc>,
}

impl ProxyState {
    pub fn state_file_path() -> Result<PathBuf> {
        Ok(app_dir()?.join("state.json"))
    }

    pub fn write_private_key(key_name: &str, material: &str) -> Result<PathBuf> {
        let keys_dir = app_dir()?.join("keys");
        fs::create_dir_all(&keys_dir)?;
        let path = keys_dir.join(format!("{}.pem", key_name));
        fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)?
            .write_all(material.as_bytes())?;
        Ok(path)
    }

    pub fn load() -> Result<Option<Self>> {
        load_json(&Self::state_file_path()?)
    }

    pub fn save(&self) -> Result<()> {
        save_json(&Self::state_file_path()?, self)
    }

    pub fn delete() -> Result<()> {
        remove_file_if_exists(&Self::state_file_path()?)?;
        Ok(())
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
            system_proxy_service: Some("Wi-Fi".to_string()),
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
        assert_eq!(
            state.system_proxy_service,
            deserialized.system_proxy_service
        );
        assert_eq!(state.started_at, deserialized.started_at);
    }

    #[test]
    fn test_deserialize_legacy_state_without_service() {
        let json = r#"{"instance_id":"i-1","region":"us-west-2","public_ip":"1.2.3.4","security_group_id":"sg-1","key_pair_name":"k","key_path":"/tmp/k.pem","local_port":1080,"ssh_pid":123,"started_at":"2024-01-15T10:30:00Z"}"#;
        let state: ProxyState = serde_json::from_str(json).unwrap();
        assert!(state.system_proxy_service.is_none());
    }

    #[test]
    fn test_state_file_path() {
        let path = ProxyState::state_file_path().unwrap();
        assert!(path.to_string_lossy().contains(".region-proxy"));
        assert!(path.to_string_lossy().ends_with("state.json"));
    }
}
