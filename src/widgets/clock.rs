use chrono::Local;

/// Render the clock widget.
///
/// Returns one or two lines:
/// - `show_date = false`: `["HH:MM"]`
/// - `show_date = true`:  `["YYYY-MM-DD", "HH:MM"]`
///
/// `format` is a `strftime`-style format string for the time line
/// (e.g. `"%H:%M"` for 24-hour, `"%I:%M %p"` for 12-hour).
pub fn render_clock(format: &str, show_date: bool) -> Vec<String> {
    let now = Local::now();
    let time_str = now.format(format).to_string();
    if show_date {
        vec![now.format("%Y-%m-%d").to_string(), time_str]
    } else {
        vec![time_str]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_clock_no_date_returns_one_line() {
        let lines = render_clock("%H:%M", false);
        assert_eq!(lines.len(), 1);
        // HH:MM — 5 chars
        assert_eq!(lines[0].len(), 5);
    }

    #[test]
    fn test_render_clock_with_date_returns_two_lines() {
        let lines = render_clock("%H:%M", true);
        assert_eq!(lines.len(), 2);
        // YYYY-MM-DD — 10 chars
        assert_eq!(lines[0].len(), 10);
        // HH:MM — 5 chars
        assert_eq!(lines[1].len(), 5);
    }

    #[test]
    fn test_render_clock_date_format_is_iso() {
        let lines = render_clock("%H:%M", true);
        // Must match YYYY-MM-DD
        let re = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
        assert!(re.is_match(&lines[0]), "date line: {:?}", lines[0]);
    }

    #[test]
    fn test_render_clock_time_format_24h() {
        let lines = render_clock("%H:%M", false);
        let re = regex::Regex::new(r"^\d{2}:\d{2}$").unwrap();
        assert!(re.is_match(&lines[0]), "time line: {:?}", lines[0]);
    }

    #[test]
    fn test_render_clock_custom_format() {
        // Seconds included — length should be 8 (HH:MM:SS)
        let lines = render_clock("%H:%M:%S", false);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 8);
    }
}
