use crate::issue::IssueWithId;

/// Fuzzy match a query string against a tag
///
/// Matching rules:
/// - Case-insensitive
/// - Query is a substring of tag
/// - Examples: "sec" matches "security", "feat" matches "feature"
pub fn fuzzy_match_tag(query: &str, tag: &str) -> bool {
   tag.to_lowercase().contains(&query.to_lowercase())
}

/// Filter issues by tags using fuzzy matching
///
/// All tag queries must match at least one tag in the issue (AND logic across
/// queries)
pub fn filter_by_tags(issues: Vec<IssueWithId>, tag_queries: &[String]) -> Vec<IssueWithId> {
   if tag_queries.is_empty() {
      return issues;
   }

   issues
      .into_iter()
      .filter(|issue_with_id| {
         tag_queries.iter().all(|query| {
            issue_with_id
               .issue
               .metadata
               .tags
               .iter()
               .any(|tag| fuzzy_match_tag(query, tag))
         })
      })
      .collect()
}

/// Exact match issues by tags (no fuzzy matching)
pub fn filter_by_tags_exact(issues: Vec<IssueWithId>, tags: &[String]) -> Vec<IssueWithId> {
   if tags.is_empty() {
      return issues;
   }

   issues
      .into_iter()
      .filter(|issue_with_id| {
         tags.iter().all(|tag| {
            issue_with_id
               .issue
               .metadata
               .tags
               .iter()
               .any(|t| t.eq_ignore_ascii_case(tag))
         })
      })
      .collect()
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn test_fuzzy_match_tag() {
      assert!(fuzzy_match_tag("sec", "security"));
      assert!(fuzzy_match_tag("feat", "feature"));
      assert!(fuzzy_match_tag("bug", "bugfix"));
      assert!(fuzzy_match_tag("SEC", "security"));
      assert!(!fuzzy_match_tag("xyz", "security"));
   }
}
