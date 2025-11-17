use anyhow::Result;

/// Parse effort string like "2h", "30m", "1d" into minutes
pub fn parse_effort(s: &str) -> Result<u32> {
   let s = s.trim();

   if s.is_empty() {
      anyhow::bail!("Empty effort string");
   }

   // Find where the number ends and unit begins
   let mut num_end = 0;
   for (i, c) in s.chars().enumerate() {
      if c.is_ascii_digit() || c == '.' {
         num_end = i + 1;
      } else {
         break;
      }
   }

   if num_end == 0 {
      anyhow::bail!("No number found in effort string: {s}");
   }

   let num_part = &s[..num_end];
   let unit_part = s[num_end..].trim();

   let value: f64 = num_part
      .parse()
      .map_err(|_| anyhow::anyhow!("Invalid number in effort: {num_part}"))?;

   let minutes = match unit_part {
      "m" | "min" | "mins" | "minute" | "minutes" => value,
      "h" | "hr" | "hrs" | "hour" | "hours" => value * 60.0,
      "d" | "day" | "days" => value * 60.0 * 8.0, // 8-hour workday
      "w" | "week" | "weeks" => value * 60.0 * 8.0 * 5.0, // 5-day work week
      "" => value,                                // Default to minutes if no unit
      _ => anyhow::bail!("Unknown effort unit: {unit_part}"),
   };

   Ok(minutes as u32)
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn test_parse_effort() {
      assert_eq!(parse_effort("30m").unwrap(), 30);
      assert_eq!(parse_effort("2h").unwrap(), 120);
      assert_eq!(parse_effort("1d").unwrap(), 480);
      assert_eq!(parse_effort("0.5h").unwrap(), 30);
      assert_eq!(parse_effort("1.5 hours").unwrap(), 90);
   }
}
