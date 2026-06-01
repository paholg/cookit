//! Parsing & display for the optional per-step countdown duration.
//!
//! Input is free-form (`30s`, `5m`, `1h 30m`, `1h30m`). We let `jiff` do the
//! heavy lifting — it understands both the "friendly" and ISO 8601 forms — and
//! then expose two display helpers: one compact label for the form/button, and
//! a clock-style countdown for the running timer bar.

use {jiff::SignedDuration, std::str::FromStr};

/// Parse user input into a positive number of seconds. Returns a human-readable
/// error suitable for inline display next to the field.
pub fn parse_duration(text: &str) -> Result<i64, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("enter a duration like `30s` or `1h 30m`".to_string());
    }

    let d = SignedDuration::from_str(trimmed)
        .map_err(|_| format!("`{trimmed}` is not a duration (try `30s`, `5m`, `1h 30m`)"))?;

    let secs = d.as_secs();
    if secs <= 0 {
        return Err("duration must be greater than zero".to_string());
    }
    Ok(secs)
}

/// Compact label like `1h 30m`, `45m`, `30s`. Used in the recipe form (after a
/// successful blur-parse) and on the start-timer button next to the hourglass.
pub fn format_duration(seconds: i64) -> String {
    let s = seconds.unsigned_abs();
    let hours = s / 3600;
    let mins = (s % 3600) / 60;
    let secs = s % 60;

    let mut parts: Vec<String> = Vec::new();
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if mins > 0 {
        parts.push(format!("{mins}m"));
    }
    // Only include seconds when they're the only unit, or there's a non-zero
    // sub-minute remainder. `1h 0m 30s` reads worse than `1h 30s`.
    if secs > 0 || parts.is_empty() {
        parts.push(format!("{secs}s"));
    }
    parts.join(" ")
}

/// Clock-style countdown for the timer bar. `mm:ss` under an hour, `h:mm:ss`
/// otherwise. Negative input (expired timer) gets a leading `-`.
pub fn format_countdown(seconds: i64) -> String {
    let sign = if seconds < 0 { "-" } else { "" };
    let s = seconds.unsigned_abs();
    let hours = s / 3600;
    let mins = (s % 3600) / 60;
    let secs = s % 60;

    if hours > 0 {
        format!("{sign}{hours}:{mins:02}:{secs:02}")
    } else {
        format!("{sign}{mins}:{secs:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_friendly_forms() {
        assert_eq!(parse_duration("30s"), Ok(30));
        assert_eq!(parse_duration("5m"), Ok(300));
        assert_eq!(parse_duration("1h"), Ok(3600));
        assert_eq!(parse_duration("1h 30m"), Ok(5400));
        assert_eq!(parse_duration("1h30m"), Ok(5400));
        assert_eq!(parse_duration("90s"), Ok(90));
    }

    #[test]
    fn rejects_garbage_and_zero() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("zzz").is_err());
        assert!(parse_duration("0s").is_err());
        assert!(parse_duration("-5m").is_err());
    }

    #[test]
    fn formats_compactly() {
        assert_eq!(format_duration(30), "30s");
        assert_eq!(format_duration(300), "5m");
        assert_eq!(format_duration(3600), "1h");
        assert_eq!(format_duration(5400), "1h 30m");
        assert_eq!(format_duration(5430), "1h 30m 30s");
        assert_eq!(format_duration(3630), "1h 30s");
    }

    #[test]
    fn countdown_format() {
        assert_eq!(format_countdown(0), "0:00");
        assert_eq!(format_countdown(65), "1:05");
        assert_eq!(format_countdown(3725), "1:02:05");
        assert_eq!(format_countdown(-5), "-0:05");
        assert_eq!(format_countdown(-3725), "-1:02:05");
    }
}
