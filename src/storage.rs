use crate::issue::{Issue, IssueMetadata};
use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const ISSUES_DIR: &str = "issues";
const OPEN_DIR: &str = "issues/open";
const CLOSED_DIR: &str = "issues/closed";
const ALIASES_FILE: &str = "issues/.aliases.yaml";

pub struct Storage {
    base_dir: PathBuf,
}

impl Storage {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    fn issues_dir(&self) -> PathBuf {
        self.base_dir.join(ISSUES_DIR)
    }

    fn open_dir(&self) -> PathBuf {
        self.base_dir.join(OPEN_DIR)
    }

    fn closed_dir(&self) -> PathBuf {
        self.base_dir.join(CLOSED_DIR)
    }

    fn aliases_file(&self) -> PathBuf {
        self.base_dir.join(ALIASES_FILE)
    }

    pub fn load_aliases(&self) -> Result<HashMap<String, u32>> {
        let path = self.aliases_file();
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let content = fs::read_to_string(&path)?;
        Ok(serde_yaml::from_str(&content).unwrap_or_default())
    }

    pub fn save_aliases(&self, aliases: &HashMap<String, u32>) -> Result<()> {
        fs::create_dir_all(self.issues_dir())?;
        let content = serde_yaml::to_string(aliases)?;
        fs::write(self.aliases_file(), content)?;
        Ok(())
    }

    pub fn resolve_bug_ref(&self, bug_ref: &str) -> Result<u32> {
        // Try parsing as number
        if let Ok(num) = bug_ref.parse::<u32>() {
            return Ok(num);
        }

        // Try resolving as alias
        let aliases = self.load_aliases()?;
        aliases
            .get(bug_ref)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("Unknown bug reference: {bug_ref}"))
    }

    pub fn parse_mdx(&self, content: &str) -> Result<(IssueMetadata, String)> {
        let re = Regex::new(r"(?s)^---\s*\n(.*?)\n---\s*\n(.*)").unwrap();

        if let Some(caps) = re.captures(content) {
            let yaml_text = &caps[1];
            let body = caps[2].to_string();

            let metadata: IssueMetadata = serde_yaml::from_str(yaml_text)
                .context("Failed to parse YAML frontmatter")?;

            Ok((metadata, body))
        } else {
            anyhow::bail!("Invalid MDX format: missing frontmatter")
        }
    }

    pub fn find_issue_file(&self, bug_num: u32) -> Result<PathBuf> {
        let padded = format!("{bug_num:02}");

        for dir in [self.open_dir(), self.closed_dir()] {
            if !dir.exists() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();

                    if name_str.starts_with(&format!("{padded}-"))
                        && (name_str.ends_with(".mdx") || name_str.ends_with(".md"))
                    {
                        return Ok(entry.path());
                    }
                }
            }
        }

        anyhow::bail!("BUG-{bug_num} not found")
    }

    pub fn load_issue(&self, bug_num: u32) -> Result<Issue> {
        let path = self.find_issue_file(bug_num)?;
        let content = fs::read_to_string(&path)?;
        let (metadata, body) = self.parse_mdx(&content)?;

        Ok(Issue { metadata, body })
    }

    pub fn next_bug_number(&self) -> Result<u32> {
        let mut max_num = 0u32;

        for dir in [self.open_dir(), self.closed_dir()] {
            if !dir.exists() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();

                    if let Some(caps) = Regex::new(r"^(\d+)-")?.captures(&name_str) {
                        if let Ok(num) = caps[1].parse::<u32>() {
                            max_num = max_num.max(num);
                        }
                    }
                }
            }
        }

        Ok(max_num + 1)
    }

    pub fn slugify(title: &str) -> String {
        let re = Regex::new(r"[^a-zA-Z0-9]+").unwrap();
        let lower = title.trim().to_lowercase();
        let slug = re.replace_all(&lower, "-");
        slug.trim_matches('-').to_string()
    }

    pub fn save_issue(&self, issue: &Issue, is_open: bool) -> Result<PathBuf> {
        let dir = if is_open {
            self.open_dir()
        } else {
            self.closed_dir()
        };
        fs::create_dir_all(&dir)?;

        let slug = Self::slugify(&issue.metadata.title);
        let filename = format!("{:02}-{slug}.mdx", issue.metadata.id);
        let path = dir.join(filename);

        fs::write(&path, issue.to_mdx())?;
        Ok(path)
    }

    pub fn update_issue_metadata<F>(&self, bug_num: u32, update_fn: F) -> Result<()>
    where
        F: FnOnce(&mut IssueMetadata),
    {
        let path = self.find_issue_file(bug_num)?;
        let content = fs::read_to_string(&path)?;
        let (mut metadata, body) = self.parse_mdx(&content)?;

        update_fn(&mut metadata);

        let issue = Issue { metadata, body };
        fs::write(&path, issue.to_mdx())?;

        Ok(())
    }

    pub fn move_issue(&self, bug_num: u32, to_open: bool) -> Result<PathBuf> {
        let src_path = self.find_issue_file(bug_num)?;
        let content = fs::read_to_string(&src_path)?;
        let (metadata, body) = self.parse_mdx(&content)?;

        let issue = Issue { metadata, body };
        let dest_path = self.save_issue(&issue, to_open)?;

        fs::remove_file(src_path)?;
        Ok(dest_path)
    }

    pub fn list_open_issues(&self) -> Result<Vec<Issue>> {
        self.list_issues_in_dir(&self.open_dir())
    }

    pub fn list_closed_issues(&self) -> Result<Vec<Issue>> {
        self.list_issues_in_dir(&self.closed_dir())
    }

    fn list_issues_in_dir(&self, dir: &Path) -> Result<Vec<Issue>> {
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut issues = Vec::new();
        let re = Regex::new(r"^(\d+)-.*\.mdx?$").unwrap();

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if re.captures(&name_str).is_some() {
                let content = fs::read_to_string(entry.path())?;
                let (metadata, body) = self.parse_mdx(&content)?;
                issues.push(Issue { metadata, body });
            }
        }

        issues.sort_by_key(|issue| issue.metadata.id);
        Ok(issues)
    }
}
