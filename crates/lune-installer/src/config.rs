//! Configuration file parsing for lune.config.json

use serde::Deserialize;
use std::collections::HashMap;

/// Root configuration structure.
#[derive(Debug, Deserialize)]
pub struct LuneConfig {
    /// Package dependencies.
    pub packages: HashMap<String, PackageSpec>,

    /// Path aliases for require().
    #[serde(default)]
    pub aliases: HashMap<String, String>,

    /// Custom registry URL (optional).
    #[serde(default)]
    pub registry: Option<String>,
}

/// Single package specification.
#[derive(Debug, Deserialize)]
pub struct PackageSpec {
    /// Source URL (e.g., "github:user/repo" or full URL).
    pub source: String,

    /// Version constraint (semver format: "^1.0.0", "~2.1", ">=1.0").
    pub version: String,

    /// Subpath within the repository (optional).
    #[serde(default)]
    pub path: Option<String>,

    /// Specific branch to use instead of tags.
    #[serde(default)]
    pub branch: Option<String>,
}

impl LuneConfig {
    /// Parse config from JSON string.
    pub fn from_str(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_config() {
        let json = r#"{
            "packages": {
                "discord": {
                    "source": "github:user/discord-luau",
                    "version": "^1.0.0"
                }
            }
        }"#;

        let config: LuneConfig = serde_json::from_str(json).unwrap();
        assert!(config.packages.contains_key("discord"));
    }
}
