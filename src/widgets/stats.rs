#[derive(Clone, Debug, Default)]
pub struct StatsData {
    pub cpu_pct: Option<f64>,
    pub mem_used_gb: Option<f64>,
    pub mem_total_gb: Option<f64>,
    pub battery_pct: Option<u8>,
}

pub fn parse_stats_output(output: &str) -> StatsData {
    let mut data = StatsData::default();
    for token in output.trim().split_whitespace() {
        if let Some((key, val)) = token.split_once('=') {
            match key {
                "cpu" => data.cpu_pct = val.parse().ok(),
                "mem" => {
                    if let Some((u, t)) = val.split_once('/') {
                        data.mem_used_gb = u.parse().ok();
                        data.mem_total_gb = t.parse().ok();
                    }
                }
                "bat" => data.battery_pct = val.parse().ok(),
                _ => {}
            }
        }
    }
    data
}

pub fn render_stats(stats: &Option<StatsData>) -> String {
    let s = match stats {
        Some(s) => s,
        None => return String::new(),
    };
    let mut parts = Vec::new();
    if let Some(c) = s.cpu_pct {
        parts.push(format!("CPU:{:.0}%", c));
    }
    if let Some(u) = s.mem_used_gb {
        parts.push(format!("MEM:{:.1}G", u));
    }
    if let Some(b) = s.battery_pct {
        parts.push(format!("BAT:{}%", b));
    }
    if parts.is_empty() {
        String::new()
    } else {
        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full() {
        let d = parse_stats_output("cpu=23 mem=4.2/16.0 bat=87");
        assert_eq!(d.cpu_pct, Some(23.0));
        assert_eq!(d.mem_used_gb, Some(4.2));
        assert_eq!(d.mem_total_gb, Some(16.0));
        assert_eq!(d.battery_pct, Some(87));
    }
    #[test]
    fn test_parse_empty() {
        let d = parse_stats_output("");
        assert!(d.cpu_pct.is_none());
    }
    #[test]
    fn test_parse_partial() {
        let d = parse_stats_output("cpu=50 bat=92");
        assert_eq!(d.cpu_pct, Some(50.0));
        assert!(d.mem_used_gb.is_none());
        assert_eq!(d.battery_pct, Some(92));
    }
    #[test]
    fn test_parse_malformed() {
        let d = parse_stats_output("cpu=abc mem=x/y");
        assert!(d.cpu_pct.is_none());
        assert!(d.mem_used_gb.is_none());
    }
    #[test]
    fn test_render_all() {
        let d = Some(StatsData {
            cpu_pct: Some(23.0),
            mem_used_gb: Some(4.2),
            mem_total_gb: Some(16.0),
            battery_pct: Some(87),
        });
        let s = render_stats(&d);
        assert!(s.contains("CPU:23%"));
        assert!(s.contains("MEM:4.2G"));
        assert!(s.contains("BAT:87%"));
    }
    #[test]
    fn test_render_none() {
        assert_eq!(render_stats(&None), "");
    }
    #[test]
    fn test_render_partial() {
        let d = Some(StatsData {
            cpu_pct: Some(10.0),
            mem_used_gb: None,
            mem_total_gb: None,
            battery_pct: None,
        });
        assert_eq!(render_stats(&d), "CPU:10%");
    }
    #[test]
    fn test_parse_zero() {
        let d = parse_stats_output("cpu=0 mem=0.0/0.0 bat=0");
        assert_eq!(d.cpu_pct, Some(0.0));
        assert_eq!(d.battery_pct, Some(0));
    }
}
