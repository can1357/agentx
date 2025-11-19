use std::{
   env, fs,
   io::Write as _,
   path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde_json::json;

const SAFE_TOOLS: &[&str] = &[
   "issues/context",
   "issues/create",
   "issues/status",
   "issues/show",
   "issues/checkpoint",
   "issues/wins",
   "issues/search",
   "issues/query",
];

/// Get the MCP server config for stdio transport
fn get_mcp_config(exe_path: &Path) -> Result<serde_json::Value> {
   Ok(json!({
      "agentx": {
         "command": exe_path.to_str().context("Invalid executable path")?,
         "args": ["serve"],
         "autoApprove": SAFE_TOOLS,
         "alwaysAllow": SAFE_TOOLS,
      }
   }))
}

/// Get the MCP server config for TOML-based clients (Codex)
fn get_mcp_config_toml(exe_path: &Path) -> Result<String> {
   Ok(format!(
      r#"
[mcp_servers.agentx]
command = "{}"
args = ["serve"]
"#,
      exe_path.to_str().context("Invalid executable path")?
   ))
}

/// Install MCP server configuration for supported clients
pub fn install_mcp_servers(uninstall: bool) -> Result<()> {
   let exe_path = env::current_exe()?;

   let configs = get_client_configs();
   let mut installed = 0;

   for (name, (config_dir, config_file)) in configs {
      let config_path = config_dir.join(config_file);
      let is_toml = config_file.ends_with(".toml");

      if !config_dir.exists() {
         println!("Skipping {name} (not found at {})", config_dir.display());
         continue;
      }

      if is_toml {
         // Handle TOML files
         let mut toml_str = if !config_path.exists() {
            String::new()
         } else {
            fs::read_to_string(&config_path)?
         };

         if uninstall {
            if !toml_str.contains("[mcp_servers.agentx]") {
               println!("Skipping {name} (not installed)");
               continue;
            }
            // Simple approach: filter out lines between [mcp_servers.agentx] and next
            // section
            let mut result = String::new();
            let mut skip = false;
            for line in toml_str.lines() {
               if line.trim() == "[mcp_servers.agentx]" {
                  skip = true;
                  continue;
               }
               if skip && line.trim_start().starts_with('[') {
                  skip = false;
               }
               if !skip {
                  result.push_str(line);
                  result.push('\n');
               }
            }
            fs::write(&config_path, result.as_bytes())?;
         } else {
            if toml_str.contains("[mcp_servers.agentx]") {
               println!("Skipping {name} (already installed)");
               continue;
            }
            // Append the new config
            toml_str.push_str(&get_mcp_config_toml(&exe_path)?);
            fs::write(&config_path, toml_str.as_bytes())?;
         }

         println!(
            "{} {name} MCP server (restart required)",
            if uninstall {
               "Uninstalled"
            } else {
               "Installed"
            }
         );
         println!("  Config: {}", config_path.display());
         installed += 1;
      } else {
         // Handle JSON files
         let mut config = if !config_path.exists() {
            json!({})
         } else {
            let data = fs::read_to_string(&config_path)?;
            if data.trim().is_empty() {
               json!({})
            } else {
               serde_json::from_str(&data)
                  .with_context(|| format!("Failed to parse config at {}", config_path.display()))?
            }
         };

         let obj = config
            .as_object_mut()
            .context("Config is not a JSON object")?;
         let mcp_servers = obj
            .entry("mcpServers")
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .context("mcpServers is not an object")?;

         if uninstall {
            if !mcp_servers.contains_key("agentx") {
               println!("Skipping {name} (not installed)");
               continue;
            }
            mcp_servers.remove("agentx");
         } else {
            if mcp_servers.contains_key("agentx") {
               println!("Skipping {name} (already installed)");
               continue;
            }
            let server_config = get_mcp_config(&exe_path)?;
            mcp_servers.insert(
               "agentx".to_string(),
               server_config
                  .get("agentx")
                  .context("Missing agentx config")?
                  .clone(),
            );
         }

         // Write updated config
         let mut file = fs::File::create(&config_path)?;
         file.write_all(serde_json::to_string_pretty(&config)?.as_bytes())?;

         println!(
            "{} {name} MCP server (restart required)",
            if uninstall {
               "Uninstalled"
            } else {
               "Installed"
            }
         );
         println!("  Config: {}", config_path.display());
         installed += 1;
      }
   }

   if installed == 0 {
      if uninstall {
         println!("No MCP servers were uninstalled");
      } else {
         println!("No supported MCP clients found");
         println!("\nFor manual installation, add this to your MCP client config:");
         println!("\n{}", serde_json::to_string_pretty(&get_mcp_config(&exe_path)?)?);
      }
   }

   Ok(())
}

#[cfg(target_os = "windows")]
fn get_client_configs() -> Vec<(&'static str, (PathBuf, &'static str))> {
   let appdata = env::var("APPDATA").unwrap_or_default();
   let home = dirs::home_dir().unwrap_or_default();

   vec![
      (
         "Cline",
         (
            PathBuf::from(&appdata)
               .join("Code")
               .join("User")
               .join("globalStorage")
               .join("saoudrizwan.claude-dev")
               .join("settings"),
            "cline_mcp_settings.json",
         ),
      ),
      (
         "Roo Code",
         (
            PathBuf::from(&appdata)
               .join("Code")
               .join("User")
               .join("globalStorage")
               .join("rooveterinaryinc.roo-cline")
               .join("settings"),
            "mcp_settings.json",
         ),
      ),
      (
         "Kilo Code",
         (
            PathBuf::from(&appdata)
               .join("Code")
               .join("User")
               .join("globalStorage")
               .join("kilocode.kilo-code")
               .join("settings"),
            "mcp_settings.json",
         ),
      ),
      ("Claude", (PathBuf::from(&appdata).join("Claude"), "claude_desktop_config.json")),
      ("Cursor", (home.join(".cursor"), "mcp.json")),
      ("Windsurf", (home.join(".codeium").join("windsurf"), "mcp_config.json")),
      ("Claude Code", (home.clone(), ".claude.json")),
      ("LM Studio", (home.join(".lmstudio"), "mcp.json")),
      ("Codex", (home.join(".codex"), "config.toml")),
      ("Zed", (PathBuf::from(&appdata).join("Zed"), "settings.json")),
      ("Gemini CLI", (home.join(".gemini"), "settings.json")),
      ("Qwen Coder", (home.join(".qwen"), "settings.json")),
      ("Copilot CLI", (home.join(".copilot"), "mcp-config.json")),
      ("Crush", (home.clone(), "crush.json")),
      (
         "Augment Code",
         (PathBuf::from(&appdata).join("Code").join("User"), "settings.json"),
      ),
      (
         "Qodo Gen",
         (PathBuf::from(&appdata).join("Code").join("User"), "settings.json"),
      ),
      ("Antigravity IDE", (home.join(".gemini").join("antigravity"), "mcp_config.json")),
      ("Warp", (home.join(".warp"), "mcp_config.json")),
      ("Amazon Q", (home.join(".aws").join("amazonq"), "mcp_config.json")),
      ("Opencode", (home.join(".opencode"), "mcp_config.json")),
      ("Kiro", (home.join(".kiro"), "mcp_config.json")),
      ("Trae", (home.join(".trae"), "mcp_config.json")),
   ]
}

#[cfg(target_os = "macos")]
fn get_client_configs() -> Vec<(&'static str, (PathBuf, &'static str))> {
   let home = dirs::home_dir().unwrap_or_default();
   let app_support = home.join("Library").join("Application Support");

   vec![
      (
         "Cline",
         (
            app_support
               .join("Code")
               .join("User")
               .join("globalStorage")
               .join("saoudrizwan.claude-dev")
               .join("settings"),
            "cline_mcp_settings.json",
         ),
      ),
      (
         "Roo Code",
         (
            app_support
               .join("Code")
               .join("User")
               .join("globalStorage")
               .join("rooveterinaryinc.roo-cline")
               .join("settings"),
            "mcp_settings.json",
         ),
      ),
      (
         "Kilo Code",
         (
            app_support
               .join("Code")
               .join("User")
               .join("globalStorage")
               .join("kilocode.kilo-code")
               .join("settings"),
            "mcp_settings.json",
         ),
      ),
      ("Claude", (app_support.join("Claude"), "claude_desktop_config.json")),
      ("Cursor", (home.join(".cursor"), "mcp.json")),
      ("Windsurf", (home.join(".codeium").join("windsurf"), "mcp_config.json")),
      ("Claude Code", (home.clone(), ".claude.json")),
      ("LM Studio", (home.join(".lmstudio"), "mcp.json")),
      ("Codex", (home.join(".codex"), "config.toml")),
      ("Zed", (app_support.join("Zed"), "settings.json")),
      ("Gemini CLI", (home.join(".gemini"), "settings.json")),
      ("Qwen Coder", (home.join(".qwen"), "settings.json")),
      ("Copilot CLI", (home.join(".copilot"), "mcp-config.json")),
      ("Crush", (home.clone(), "crush.json")),
      ("Augment Code", (app_support.join("Code").join("User"), "settings.json")),
      ("Qodo Gen", (app_support.join("Code").join("User"), "settings.json")),
      ("Antigravity IDE", (home.join(".gemini").join("antigravity"), "mcp_config.json")),
      ("Warp", (home.join(".warp"), "mcp_config.json")),
      ("Amazon Q", (home.join(".aws").join("amazonq"), "mcp_config.json")),
      ("Opencode", (home.join(".opencode"), "mcp_config.json")),
      ("Kiro", (home.join(".kiro"), "mcp_config.json")),
      ("Trae", (home.join(".trae"), "mcp_config.json")),
      ("BoltAI", (app_support.join("BoltAI"), "config.json")),
      ("Perplexity", (app_support.join("Perplexity"), "mcp_config.json")),
   ]
}

#[cfg(target_os = "linux")]
fn get_client_configs() -> Vec<(&'static str, (PathBuf, &'static str))> {
   let home = dirs::home_dir().unwrap_or_default();
   let config_dir = home.join(".config");

   vec![
      (
         "Cline",
         (
            config_dir
               .join("Code")
               .join("User")
               .join("globalStorage")
               .join("saoudrizwan.claude-dev")
               .join("settings"),
            "cline_mcp_settings.json",
         ),
      ),
      (
         "Roo Code",
         (
            config_dir
               .join("Code")
               .join("User")
               .join("globalStorage")
               .join("rooveterinaryinc.roo-cline")
               .join("settings"),
            "mcp_settings.json",
         ),
      ),
      (
         "Kilo Code",
         (
            config_dir
               .join("Code")
               .join("User")
               .join("globalStorage")
               .join("kilocode.kilo-code")
               .join("settings"),
            "mcp_settings.json",
         ),
      ),
      ("Cursor", (home.join(".cursor"), "mcp.json")),
      ("Windsurf", (home.join(".codeium").join("windsurf"), "mcp_config.json")),
      ("Claude Code", (home.clone(), ".claude.json")),
      ("LM Studio", (home.join(".lmstudio"), "mcp.json")),
      ("Codex", (home.join(".codex"), "config.toml")),
      ("Zed", (config_dir.join("zed"), "settings.json")),
      ("Gemini CLI", (home.join(".gemini"), "settings.json")),
      ("Qwen Coder", (home.join(".qwen"), "settings.json")),
      ("Copilot CLI", (home.join(".copilot"), "mcp-config.json")),
      ("Crush", (home.clone(), "crush.json")),
      ("Augment Code", (config_dir.join("Code").join("User"), "settings.json")),
      ("Qodo Gen", (config_dir.join("Code").join("User"), "settings.json")),
      ("Antigravity IDE", (home.join(".gemini").join("antigravity"), "mcp_config.json")),
      ("Warp", (home.join(".warp"), "mcp_config.json")),
      ("Amazon Q", (home.join(".aws").join("amazonq"), "mcp_config.json")),
      ("Opencode", (home.join(".opencode"), "mcp_config.json")),
      ("Kiro", (home.join(".kiro"), "mcp_config.json")),
      ("Trae", (home.join(".trae"), "mcp_config.json")),
   ]
}
