use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::store::{app_dir, load_json, remove_file_if_exists, save_json};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_region: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_port: Option<u16>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_instance_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_system_proxy: Option<bool>,
}

impl Preferences {
    pub fn config_file_path() -> Result<PathBuf> {
        Ok(app_dir()?.join("config.json"))
    }

    pub fn load() -> Result<Self> {
        Ok(load_json(&Self::config_file_path()?)?.unwrap_or_default())
    }

    pub fn update(f: impl FnOnce(&mut Self)) -> Result<()> {
        let mut prefs = Self::load()?;
        f(&mut prefs);
        save_json(&Self::config_file_path()?, &prefs)
    }

    /// Returns true if a config file existed and was removed.
    pub fn delete() -> Result<bool> {
        remove_file_if_exists(&Self::config_file_path()?)
    }

    pub fn is_empty(&self) -> bool {
        self.default_region.is_none()
            && self.default_port.is_none()
            && self.default_instance_type.is_none()
            && self.no_system_proxy.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_preferences() {
        let prefs = Preferences::default();
        assert!(prefs.default_region.is_none());
        assert!(prefs.default_port.is_none());
        assert!(prefs.default_instance_type.is_none());
        assert!(prefs.no_system_proxy.is_none());
        assert!(prefs.is_empty());
    }

    #[test]
    fn test_serialize_deserialize() {
        let prefs = Preferences {
            default_region: Some("ap-northeast-1".to_string()),
            default_port: Some(8080),
            default_instance_type: Some("t4g.micro".to_string()),
            no_system_proxy: Some(true),
        };

        let json = serde_json::to_string(&prefs).unwrap();
        let deserialized: Preferences = serde_json::from_str(&json).unwrap();

        assert_eq!(prefs.default_region, deserialized.default_region);
        assert_eq!(prefs.default_port, deserialized.default_port);
        assert_eq!(
            prefs.default_instance_type,
            deserialized.default_instance_type
        );
        assert_eq!(prefs.no_system_proxy, deserialized.no_system_proxy);
    }

    #[test]
    fn test_serialize_empty_preferences() {
        let prefs = Preferences::default();
        let json = serde_json::to_string(&prefs).unwrap();
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_serialize_partial_preferences() {
        let prefs = Preferences {
            default_region: Some("us-west-2".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&prefs).unwrap();
        assert!(json.contains("default_region"));
        assert!(json.contains("us-west-2"));
        assert!(!json.contains("default_port"));
    }

    #[test]
    fn test_is_empty() {
        let mut prefs = Preferences::default();
        assert!(prefs.is_empty());

        prefs.default_region = Some("ap-northeast-1".to_string());
        assert!(!prefs.is_empty());
    }

    #[test]
    fn test_config_file_path() {
        let path = Preferences::config_file_path().unwrap();
        assert!(path.to_string_lossy().contains(".region-proxy"));
        assert!(path.to_string_lossy().ends_with("config.json"));
    }
}
