use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

mod datetime_rfc3339 {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&date.to_rfc3339())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
    }
}

mod datetime_rfc3339_option {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(dt) => serializer.serialize_str(&dt.to_rfc3339()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        match opt {
            Some(s) => DateTime::parse_from_rfc3339(&s)
                .map(|dt| Some(dt.with_timezone(&Utc)))
                .map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[serde(rename = "open")]
    NotStarted,
    #[serde(rename = "active")]
    InProgress,
    Blocked,
    Done,
    Closed,
    Backlog,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotStarted => write!(f, "open"),
            Self::InProgress => write!(f, "active"),
            Self::Blocked => write!(f, "blocked"),
            Self::Done => write!(f, "done"),
            Self::Closed => write!(f, "closed"),
            Self::Backlog => write!(f, "backlog"),
        }
    }
}

impl Status {
    pub fn marker(&self) -> &'static str {
        match self {
            Self::NotStarted => "⭕",
            Self::InProgress => "🟡",
            Self::Blocked => "🚫",
            Self::Done => "🟢",
            Self::Closed => "🗑️",
            Self::Backlog => "💤",
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
    #[serde(with = "datetime_rfc3339")]
    pub created: DateTime<Utc>,
    pub files: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub context: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "datetime_rfc3339_option",
        default
    )]
    pub started: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub blocked_reason: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "datetime_rfc3339_option",
        default
    )]
    pub closed: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub depends_on: Vec<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub blocks: Vec<u32>,
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
            depends_on: Vec::new(),
            blocks: Vec::new(),
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
