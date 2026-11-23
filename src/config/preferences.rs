use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::store::{app_dir, load_json, remove_file_if_exists, save_json};

pub const DEFAULT_PORT: u16 = 1080;
pub const DEFAULT_INSTANCE_TYPE: &str = "t4g.nano";

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
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
        *self == Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_empty_preferences() {
        let prefs = Preferences::default();
        assert!(prefs.is_empty());
        assert_eq!(serde_json::to_string(&prefs).unwrap(), "{}");
    }
}
