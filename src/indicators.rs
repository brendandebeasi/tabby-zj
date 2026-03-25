use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct IndicatorState {
    pub busy: bool,
    pub bell: bool,
    pub input: bool,
    pub busy_frame: usize,
}

pub const BUSY_FRAMES: &[char] = &['◐', '◓', '◑', '◒'];

pub fn render_indicators(state: &IndicatorState) -> String {
    let mut s = String::new();
    if state.busy {
        s.push(BUSY_FRAMES[state.busy_frame % BUSY_FRAMES.len()]);
    }
    if state.bell {
        s.push('◆');
    }
    if state.input {
        s.push('?');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_indicators_empty_when_all_false() {
        let state = IndicatorState::default();
        assert_eq!(render_indicators(&state), "");
    }

    #[test]
    fn test_render_indicators_busy_shows_frame() {
        let state = IndicatorState {
            busy: true,
            busy_frame: 0,
            ..IndicatorState::default()
        };
        let s = render_indicators(&state);
        assert_eq!(s, "◐");
    }

    #[test]
    fn test_render_indicators_bell_shows_diamond() {
        let state = IndicatorState {
            bell: true,
            ..IndicatorState::default()
        };
        assert!(render_indicators(&state).contains('◆'));
    }

    #[test]
    fn test_render_indicators_input_shows_question() {
        let state = IndicatorState {
            input: true,
            ..IndicatorState::default()
        };
        assert!(render_indicators(&state).contains('?'));
    }

    #[test]
    fn test_render_indicators_multiple_combined() {
        let state = IndicatorState {
            bell: true,
            input: true,
            ..IndicatorState::default()
        };
        let s = render_indicators(&state);
        assert!(s.contains('◆'));
        assert!(s.contains('?'));
    }

    #[test]
    fn test_busy_frames_cycle() {
        assert_eq!(BUSY_FRAMES.len(), 4);
        let f0 = IndicatorState {
            busy: true,
            busy_frame: 0,
            ..IndicatorState::default()
        };
        let f1 = IndicatorState {
            busy: true,
            busy_frame: 1,
            ..IndicatorState::default()
        };
        assert_ne!(render_indicators(&f0), render_indicators(&f1));
    }
}
