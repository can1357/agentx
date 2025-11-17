use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

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

   #[serde(default = "default_colored_output")]
   pub colored_output: bool,

   #[serde(default = "default_issue_prefix")]
   pub issue_prefix: String,

   #[serde(default)]
   pub git_integration: GitIntegration,

   #[serde(default)]
   pub templates_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitIntegration {
   #[serde(default)]
   pub enabled: bool,

   #[serde(default = "default_branch_prefix")]
   pub branch_prefix: String,

   #[serde(default)]
   pub commit_prefix_format: Option<String>,

   #[serde(default)]
   pub auto_branch: bool,
}

impl Default for GitIntegration {
   fn default() -> Self {
      Self {
         enabled:              false,
         branch_prefix:        default_branch_prefix(),
         commit_prefix_format: None,
         auto_branch:          false,
      }
   }
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

fn default_colored_output() -> bool {
   true
}

fn default_issue_prefix() -> String {
   "ISSUE".to_string()
}

fn default_branch_prefix() -> String {
   "issue-".to_string()
}

impl Default for Config {
   fn default() -> Self {
      Self {
         default_priority:      default_priority(),
         default_effort_unit:   default_effort_unit(),
         auto_status_detection: true,
         issues_location:       None,
         colored_output:        default_colored_output(),
         issue_prefix:          default_issue_prefix(),
         git_integration:       GitIntegration::default(),
         templates_dir:         None,
      }
   }
}

impl Config {
   /// Get the formatted issue reference (e.g., "ISSUE-1" or "BUG-1")
   pub fn format_issue_ref(&self, num: u32) -> String {
      format!("{}-{}", self.issue_prefix, num)
   }
}

impl Config {
   /// Load config from .agentxrc.yaml
   /// Searches from current directory up to root
   pub fn load() -> Self {
      Self::find_and_load().unwrap_or_default()
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
         },
         Some(IssuesLocation::Fixed { path }) => path.clone(),
         Some(IssuesLocation::Home { folder }) => {
            if let Some(home_dir) = dirs::home_dir() {
               home_dir.join(".agentx").join(folder)
            } else {
               std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            }
         },
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
         default_priority:      "high".to_string(),
         default_effort_unit:   "days".to_string(),
         auto_status_detection: false,
         issues_location:       Some(IssuesLocation::Home { folder: "myproject".to_string() }),
         colored_output:        true,
         issue_prefix:          "ISSUE".to_string(),
         git_integration:       GitIntegration::default(),
         templates_dir:         None,
      };

      let yaml = serde_yaml::to_string(&config).unwrap();
      assert!(yaml.contains("high"));
      assert!(yaml.contains("days"));
   }
}
