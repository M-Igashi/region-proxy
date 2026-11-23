use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::store::{app_dir, load_json, remove_file_if_exists, save_json};

/// Resource fields stay empty until the resource is created
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProxyState {
    pub instance_id: String,
    pub region: String,
    pub public_ip: String,
    pub security_group_id: String,
    pub key_pair_name: String,
    pub local_port: u16,
    pub system_proxy_service: Option<String>,
    pub started_at: DateTime<Utc>,
}

impl ProxyState {
    fn state_file_path() -> Result<PathBuf> {
        Ok(app_dir()?.join("state.json"))
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

    #[test]
    fn test_deserialize_legacy_state() {
        let json = r#"{"instance_id":"i-1","region":"us-west-2","public_ip":"1.2.3.4","security_group_id":"sg-1","key_pair_name":"k","key_path":"/tmp/k.pem","local_port":1080,"ssh_pid":123,"started_at":"2024-01-15T10:30:00Z"}"#;
        let state: ProxyState = serde_json::from_str(json).unwrap();
        assert_eq!(state.key_pair_name, "k");
        assert!(state.system_proxy_service.is_none());
    }
}
