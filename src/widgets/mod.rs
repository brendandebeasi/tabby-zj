pub mod clock;
pub mod git;
pub mod quota;
pub mod stats;

use crate::state::PluginState;

pub fn render_pinned_lines(state: &PluginState, cols: usize) -> Vec<String> {
    let mut lines = vec![render_pinned(state, cols)];

    if state.config.widgets.stats.enabled {
        let stats_line = stats::render_stats(&state.stats);
        if !stats_line.is_empty() {
            let visible = stats_line.chars().count();
            if visible >= cols {
                lines.push(stats_line.chars().take(cols).collect());
            } else {
                lines.push(format!("{}{}", stats_line, " ".repeat(cols - visible)));
            }
        }
    }

    if state.config.widgets.quota.enabled {
        let quota_line = quota::render_quota(&state.quota);
        if !quota_line.is_empty() {
            let visible = quota_line.chars().count();
            if visible >= cols {
                lines.push(quota_line.chars().take(cols).collect());
            } else {
                lines.push(format!("{}{}", quota_line, " ".repeat(cols - visible)));
            }
        }
    }

    if state.config.widgets.pet.enabled {
        let frame = state
            .pet_animation
            .as_ref()
            .map(|a| a.current_frame())
            .unwrap_or("(=^·ω·^=)");
        if let Some(ref pet) = state.pet_state {
            let pet_lines = crate::pet::render_pet(pet, frame, cols);
            lines.extend(pet_lines);
        }
    }

    lines
}

pub fn render_pinned(state: &PluginState, cols: usize) -> String {
    let left = if state.config.widgets.clock.enabled {
        clock::render_clock(&state.config.widgets.clock.format, false)
            .into_iter()
            .next()
            .unwrap_or_default()
    } else {
        String::new()
    };

    let right = if state.config.widgets.git.enabled {
        git::render_git(&state.git_status)
            .into_iter()
            .next()
            .unwrap_or_default()
            .trim()
            .to_string()
    } else {
        String::new()
    };

    let left_len = left.chars().count();
    let right_len = right.chars().count();

    if left.is_empty() && right.is_empty() {
        " ".repeat(cols.max(1))
    } else if right.is_empty() {
        let pad = cols.saturating_sub(left_len);
        format!("{}{}", left, " ".repeat(pad))
    } else if left.is_empty() {
        let pad = cols.saturating_sub(right_len);
        format!("{}{}", " ".repeat(pad), right)
    } else {
        let gap = cols.saturating_sub(left_len + right_len);
        format!("{}{}{}", left, " ".repeat(gap.max(1)), right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{GitStatus, PluginState};

    #[test]
    fn test_render_pinned_no_widgets_returns_spaces() {
        let mut state = PluginState::default();
        state.config.widgets.clock.enabled = false;
        state.config.widgets.git.enabled = false;
        let line = render_pinned(&state, 20);
        assert_eq!(line.len(), 20);
        assert!(line.chars().all(|c| c == ' '));
    }

    #[test]
    fn test_render_pinned_pads_to_cols() {
        let mut state = PluginState::default();
        state.config.widgets.clock.enabled = true;
        state.config.widgets.clock.format = "%H:%M".into();
        state.config.widgets.git.enabled = false;
        let line = render_pinned(&state, 30);
        assert_eq!(line.chars().count(), 30);
    }

    #[test]
    fn test_render_pinned_clock_on_left() {
        let mut state = PluginState::default();
        state.config.widgets.clock.enabled = true;
        state.config.widgets.clock.format = "%H:%M".into();
        state.config.widgets.git.enabled = false;
        let line = render_pinned(&state, 20);
        let re = regex::Regex::new(r"^\d{2}:\d{2}").expect("valid static regex pattern");
        assert!(
            re.is_match(&line),
            "clock should be on the left: {:?}",
            line
        );
    }

    #[test]
    fn test_render_pinned_git_on_right() {
        let mut state = PluginState::default();
        state.config.widgets.clock.enabled = false;
        state.config.widgets.git.enabled = true;
        state.git_status = Some(GitStatus {
            branch: "main".into(),
            ..GitStatus::default()
        });
        let line = render_pinned(&state, 20);
        assert!(
            line.contains("main"),
            "git branch should appear: {:?}",
            line
        );
        assert_eq!(line.chars().count(), 20);
    }

    #[test]
    fn test_render_pinned_clock_and_git_both_shown() {
        let mut state = PluginState::default();
        state.config.widgets.clock.enabled = true;
        state.config.widgets.clock.format = "%H:%M".into();
        state.config.widgets.git.enabled = true;
        state.git_status = Some(GitStatus {
            branch: "dev".into(),
            ..GitStatus::default()
        });
        let line = render_pinned(&state, 30);
        assert_eq!(line.chars().count(), 30);
        assert!(line.contains("dev"), "branch should appear");
        let re = regex::Regex::new(r"\d{2}:\d{2}").expect("valid static regex pattern");
        assert!(re.is_match(&line), "clock should appear");
    }

    #[test]
    fn test_render_pinned_ignores_stats_on_primary_line() {
        let mut state = PluginState::default();
        state.config.widgets.clock.enabled = false;
        state.config.widgets.git.enabled = false;
        state.config.widgets.stats.enabled = true;
        state.stats = Some(stats::StatsData {
            cpu_pct: Some(23.0),
            mem_used_gb: Some(4.2),
            mem_total_gb: Some(16.0),
            battery_pct: Some(87),
        });
        let line = render_pinned(&state, 40);
        assert!(!line.contains("CPU:"));
        assert!(!line.contains("MEM:"));
        assert!(!line.contains("BAT:"));
    }

    #[test]
    fn test_render_pinned_lines_includes_second_stats_line_when_enabled() {
        let mut state = PluginState::default();
        state.config.widgets.stats.enabled = true;
        state.stats = Some(stats::StatsData {
            cpu_pct: Some(10.0),
            mem_used_gb: Some(2.0),
            mem_total_gb: Some(16.0),
            battery_pct: Some(90),
        });
        let lines = render_pinned_lines(&state, 30);
        assert!(lines.len() >= 2);
        assert!(lines[1].contains("CPU:10%"));
    }
}
