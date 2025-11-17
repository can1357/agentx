use anyhow::{Result, anyhow};

/// Validate that input is not empty
pub fn validate_non_empty(input: &str) -> Result<()> {
   if input.trim().is_empty() {
      Err(anyhow!("Input cannot be empty"))
   } else {
      Ok(())
   }
}

/// Validate priority value
pub fn validate_priority(input: &str) -> Result<()> {
   match input.to_lowercase().as_str() {
      "critical" | "high" | "medium" | "low" => Ok(()),
      _ => Err(anyhow!("Priority must be one of: critical, high, medium, low")),
   }
}

/// Validate effort estimation
pub fn validate_effort(input: &str) -> Result<()> {
   // T-shirt sizes
   if matches!(input.to_uppercase().as_str(), "XS" | "S" | "M" | "L" | "XL") {
      return Ok(());
   }

   // Story points
   if matches!(input, "1" | "2" | "3" | "5" | "8" | "13" | "21") {
      return Ok(());
   }

   // Time estimates (e.g., "2h", "1d", "30m")
   if input.ends_with('h') || input.ends_with('d') || input.ends_with('m') {
      let num_part = &input[..input.len() - 1];
      if num_part.parse::<u32>().is_ok() {
         return Ok(());
      }
   }

   Err(anyhow!(
      "Effort must be T-shirt size (XS/S/M/L/XL), story points (1/2/3/5/8/13/21), or time (e.g., \
       2h, 1d)"
   ))
}

/// Validate file path exists
pub fn validate_file_exists(path: &str) -> Result<()> {
   if std::path::Path::new(path).exists() {
      Ok(())
   } else {
      Err(anyhow!("File does not exist: {}", path))
   }
}

/// Validate issue reference format
pub fn validate_issue_ref(input: &str) -> Result<()> {
   if input.trim().is_empty() {
      return Err(anyhow!("Issue reference cannot be empty"));
   }

   // Allow flexible formats: BUG-123, TASK-45, 123, etc.
   if input.contains(|c: char| !c.is_alphanumeric() && c != '-') {
      return Err(anyhow!("Issue reference can only contain alphanumeric characters and hyphens"));
   }

   Ok(())
}
