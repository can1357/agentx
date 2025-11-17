use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_priority")]
    pub default_priority: String,

    #[serde(default = "default_effort_unit")]
    pub default_effort_unit: String,

    #[serde(default = "default_auto_status")]
    pub auto_status_detection: bool,

    #[serde(default)]
    pub issues_location: Option<IssuesLocation>,

    #[serde(default)]
    pub colored_output: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum IssuesLocation {
    Cwd,
    Fixed { path: PathBuf },
    Home { folder: String },
}

fn default_priority() -> String {
    "medium".to_string()
}

fn default_effort_unit() -> String {
    "hours".to_string()
}

fn default_auto_status() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_priority: default_priority(),
            default_effort_unit: default_effort_unit(),
            auto_status_detection: true,
            issues_location: None,
            colored_output: true,
        }
    }
}

impl Config {
    /// Load config from .agentxrc.yaml
    /// Searches from current directory up to root
    pub fn load() -> Self {
        if let Ok(config) = Self::find_and_load() {
            config
        } else {
            Self::default()
        }
    }

    fn find_and_load() -> Result<Self> {
        let mut current_dir = std::env::current_dir()?;

        loop {
            let config_path = current_dir.join(".agentxrc.yaml");

            if config_path.exists() {
                let content = std::fs::read_to_string(&config_path)?;
                return Ok(serde_yaml::from_str(&content)?);
            }

            // Move to parent directory
            if !current_dir.pop() {
                break; // Reached root
            }
        }

        // Also check home directory
        if let Some(home_dir) = dirs::home_dir() {
            let config_path = home_dir.join(".agentxrc.yaml");
            if config_path.exists() {
                let content = std::fs::read_to_string(&config_path)?;
                return Ok(serde_yaml::from_str(&content)?);
            }
        }

        anyhow::bail!("No .agentxrc.yaml found")
    }

    pub fn resolve_issues_directory(&self) -> PathBuf {
        match &self.issues_location {
            Some(IssuesLocation::Cwd) | None => {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            }
            Some(IssuesLocation::Fixed { path }) => path.clone(),
            Some(IssuesLocation::Home { folder }) => {
                if let Some(home_dir) = dirs::home_dir() {
                    home_dir.join(".agentx").join(folder)
                } else {
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.default_priority, "medium");
        assert_eq!(config.default_effort_unit, "hours");
        assert!(config.auto_status_detection);
    }

    #[test]
    fn test_serialize_config() {
        let config = Config {
            default_priority: "high".to_string(),
            default_effort_unit: "days".to_string(),
            auto_status_detection: false,
            issues_location: Some(IssuesLocation::Home {
                folder: "myproject".to_string(),
            }),
            colored_output: true,
        };

        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("high"));
        assert!(yaml.contains("days"));
    }
}
