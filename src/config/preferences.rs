use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// User preferences for region-proxy
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Preferences {
    /// Default AWS region for proxy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_region: Option<String>,

    /// Default local port for SOCKS proxy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_port: Option<u16>,

    /// Default instance type override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_instance_type: Option<String>,

    /// Skip macOS system proxy configuration by default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_system_proxy: Option<bool>,
}

impl Preferences {
    /// Get the preferences file path
    pub fn config_file_path() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not find home directory")?;
        let config_dir = home.join(".region-proxy");
        fs::create_dir_all(&config_dir)?;
        Ok(config_dir.join("config.json"))
    }

    /// Load preferences from file
    pub fn load() -> Result<Self> {
        let path = Self::config_file_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)?;
        let prefs: Self = serde_json::from_str(&content)?;
        Ok(prefs)
    }

    /// Save preferences to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_file_path()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Check if preferences file exists
    #[allow(dead_code)]
    pub fn exists() -> Result<bool> {
        let path = Self::config_file_path()?;
        Ok(path.exists())
    }

    /// Set default region
    pub fn set_default_region(&mut self, region: Option<String>) {
        self.default_region = region;
    }

    /// Set default port
    pub fn set_default_port(&mut self, port: Option<u16>) {
        self.default_port = port;
    }

    /// Set default instance type
    pub fn set_default_instance_type(&mut self, instance_type: Option<String>) {
        self.default_instance_type = instance_type;
    }

    /// Set no_system_proxy preference
    pub fn set_no_system_proxy(&mut self, no_system_proxy: Option<bool>) {
        self.no_system_proxy = no_system_proxy;
    }

    /// Check if any preferences are set
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
        // Empty preferences should serialize to empty object
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_serialize_partial_preferences() {
        let prefs = Preferences {
            default_region: Some("us-west-2".to_string()),
            default_port: None,
            default_instance_type: None,
            no_system_proxy: None,
        };

        let json = serde_json::to_string_pretty(&prefs).unwrap();
        assert!(json.contains("default_region"));
        assert!(json.contains("us-west-2"));
        assert!(!json.contains("default_port"));
        assert!(!json.contains("default_instance_type"));
        assert!(!json.contains("no_system_proxy"));
    }

    #[test]
    fn test_is_empty() {
        let mut prefs = Preferences::default();
        assert!(prefs.is_empty());

        prefs.default_region = Some("ap-northeast-1".to_string());
        assert!(!prefs.is_empty());

        prefs.default_region = None;
        prefs.default_port = Some(1080);
        assert!(!prefs.is_empty());
    }

    #[test]
    fn test_setters() {
        let mut prefs = Preferences::default();

        prefs.set_default_region(Some("eu-west-1".to_string()));
        assert_eq!(prefs.default_region, Some("eu-west-1".to_string()));

        prefs.set_default_port(Some(9999));
        assert_eq!(prefs.default_port, Some(9999));

        prefs.set_default_instance_type(Some("t3.micro".to_string()));
        assert_eq!(
            prefs.default_instance_type,
            Some("t3.micro".to_string())
        );

        prefs.set_no_system_proxy(Some(true));
        assert_eq!(prefs.no_system_proxy, Some(true));
    }

    #[test]
    fn test_config_file_path() {
        let path = Preferences::config_file_path().unwrap();
        assert!(path.to_string_lossy().contains(".region-proxy"));
        assert!(path.to_string_lossy().ends_with("config.json"));
    }
}
