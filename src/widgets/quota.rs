/// Data received from a quota pipe message.
#[derive(Clone, Debug, Default)]
pub struct QuotaData {
    pub remaining: Option<u64>,
    pub limit: Option<u64>,
    pub resets: Option<String>,
    pub label: Option<String>,
}

/// Parse a quota pipe payload.
///
/// Expected format: `"remaining=450,limit=1000,resets=2h30m"` or
/// `"remaining=450,limit=1000,resets=2h30m,label=Claude"`.
pub fn parse_quota_data(data: &str) -> QuotaData {
    let mut qd = QuotaData::default();
    for pair in data.split(',') {
        if let Some((key, val)) = pair.split_once('=') {
            match key.trim() {
                "remaining" => qd.remaining = val.trim().parse().ok(),
                "limit" => qd.limit = val.trim().parse().ok(),
                "resets" => qd.resets = Some(val.trim().to_string()),
                "label" => qd.label = Some(val.trim().to_string()),
                _ => {}
            }
        }
    }
    qd
}

/// Render quota data as a compact single line.
///
/// Examples:
/// - `"Quota: 450/1000 (2h30m)"`
/// - `"Claude: 450/1000 (2h30m)"` (with label)
/// - `"Quota: 450/1000"` (no resets)
/// - `""` (no data)
pub fn render_quota(quota: &Option<QuotaData>) -> String {
    let q = match quota {
        Some(q) => q,
        None => return String::new(),
    };
    let (remaining, limit) = match (q.remaining, q.limit) {
        (Some(r), Some(l)) => (r, l),
        _ => return String::new(),
    };
    let label = q.label.as_deref().unwrap_or("Quota");
    match &q.resets {
        Some(r) if !r.is_empty() => format!("{}: {}/{} ({})", label, remaining, limit, r),
        _ => format!("{}: {}/{}", label, remaining, limit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full() {
        let d = parse_quota_data("remaining=450,limit=1000,resets=2h30m");
        assert_eq!(d.remaining, Some(450));
        assert_eq!(d.limit, Some(1000));
        assert_eq!(d.resets.as_deref(), Some("2h30m"));
        assert!(d.label.is_none());
    }

    #[test]
    fn test_parse_with_label() {
        let d = parse_quota_data("remaining=100,limit=500,resets=1h,label=Claude");
        assert_eq!(d.remaining, Some(100));
        assert_eq!(d.limit, Some(500));
        assert_eq!(d.resets.as_deref(), Some("1h"));
        assert_eq!(d.label.as_deref(), Some("Claude"));
    }

    #[test]
    fn test_parse_empty() {
        let d = parse_quota_data("");
        assert!(d.remaining.is_none());
        assert!(d.limit.is_none());
        assert!(d.resets.is_none());
    }

    #[test]
    fn test_parse_partial() {
        let d = parse_quota_data("remaining=200,limit=800");
        assert_eq!(d.remaining, Some(200));
        assert_eq!(d.limit, Some(800));
        assert!(d.resets.is_none());
    }

    #[test]
    fn test_parse_malformed() {
        let d = parse_quota_data("remaining=abc,limit=xyz");
        assert!(d.remaining.is_none());
        assert!(d.limit.is_none());
    }

    #[test]
    fn test_render_full() {
        let q = Some(QuotaData {
            remaining: Some(450),
            limit: Some(1000),
            resets: Some("2h30m".into()),
            label: None,
        });
        assert_eq!(render_quota(&q), "Quota: 450/1000 (2h30m)");
    }

    #[test]
    fn test_render_with_label() {
        let q = Some(QuotaData {
            remaining: Some(100),
            limit: Some(500),
            resets: Some("1h".into()),
            label: Some("Claude".into()),
        });
        assert_eq!(render_quota(&q), "Claude: 100/500 (1h)");
    }

    #[test]
    fn test_render_no_resets() {
        let q = Some(QuotaData {
            remaining: Some(450),
            limit: Some(1000),
            resets: None,
            label: None,
        });
        assert_eq!(render_quota(&q), "Quota: 450/1000");
    }

    #[test]
    fn test_render_none() {
        assert_eq!(render_quota(&None), "");
    }

    #[test]
    fn test_render_missing_fields() {
        let q = Some(QuotaData {
            remaining: Some(450),
            limit: None,
            resets: None,
            label: None,
        });
        assert_eq!(render_quota(&q), "");
    }
}
