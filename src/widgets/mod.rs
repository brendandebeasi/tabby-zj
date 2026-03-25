pub mod clock;
pub mod git;

use crate::state::PluginState;

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
}
