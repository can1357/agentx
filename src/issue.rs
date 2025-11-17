use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    NotStarted,
    InProgress,
    Blocked,
    Done,
    Closed,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotStarted => write!(f, "not_started"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Blocked => write!(f, "blocked"),
            Self::Done => write!(f, "done"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

impl Status {
    pub fn marker(&self) -> &'static str {
        match self {
            Self::NotStarted => "⭕",
            Self::InProgress => "🔄",
            Self::Blocked => "🚫",
            Self::Done => "✅",
            Self::Closed => "✅",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Critical => write!(f, "critical"),
            Self::High => write!(f, "high"),
            Self::Medium => write!(f, "medium"),
            Self::Low => write!(f, "low"),
        }
    }
}

impl Priority {
    pub fn sort_key(&self) -> u8 {
        match self {
            Self::Critical => 0,
            Self::High => 1,
            Self::Medium => 2,
            Self::Low => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueMetadata {
    pub id: u32,
    pub title: String,
    pub priority: Priority,
    pub status: Status,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created: DateTime<Utc>,
    pub files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub context: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "chrono::serde::ts_seconds_option",
        default
    )]
    pub started: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub blocked_reason: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "chrono::serde::ts_seconds_option",
        default
    )]
    pub closed: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub metadata: IssueMetadata,
    pub body: String,
}

impl Issue {
    pub fn new(
        id: u32,
        title: String,
        priority: Priority,
        files: Vec<String>,
        issue: String,
        impact: String,
        acceptance: String,
        effort: Option<String>,
        context: Option<String>,
    ) -> Self {
        let metadata = IssueMetadata {
            id,
            title: title.clone(),
            priority,
            status: Status::NotStarted,
            created: Utc::now(),
            files,
            effort,
            context,
            started: None,
            blocked_reason: None,
            closed: None,
        };

        let mut body = format!("# BUG-{id}: {title}\n\n");
        if !issue.is_empty() {
            body.push_str(&format!("**Issue**: {issue}\n\n"));
        }
        if !impact.is_empty() {
            body.push_str(&format!("**Impact**: {impact}\n\n"));
        }
        if !acceptance.is_empty() {
            body.push_str(&format!("**Acceptance**: {acceptance}\n\n"));
        }

        Self { metadata, body }
    }

    pub fn to_mdx(&self) -> String {
        let yaml = serde_yaml::to_string(&self.metadata).unwrap_or_default();
        format!("---\n{yaml}---\n\n{}", self.body)
    }
}
