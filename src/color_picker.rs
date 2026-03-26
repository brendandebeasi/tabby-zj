use crate::colors::{ansi_bg, ansi_fg, hsl_to_hex, Hsl};

#[derive(Clone, Debug, PartialEq)]
pub enum HslComponent {
    H,
    S,
    L,
}

#[derive(Clone)]
pub struct ColorPickerState {
    pub hsl: Hsl,
    pub active: HslComponent,
    pub target_tab: usize,
}

impl ColorPickerState {
    pub fn new(target_tab: usize, initial_hex: Option<&str>) -> Self {
        let hsl = initial_hex
            .and_then(|h| crate::colors::hex_to_hsl(h))
            .unwrap_or(Hsl {
                h: 180.0,
                s: 0.5,
                l: 0.5,
            });
        Self {
            hsl,
            active: HslComponent::H,
            target_tab,
        }
    }

    pub fn current_hex(&self) -> String {
        hsl_to_hex(&self.hsl)
    }

    pub fn adjust(&mut self, delta: f64) {
        match self.active {
            HslComponent::H => {
                self.hsl.h = (self.hsl.h + delta).rem_euclid(360.0);
            }
            HslComponent::S => {
                self.hsl.s = (self.hsl.s + delta / 100.0).clamp(0.0, 1.0);
            }
            HslComponent::L => {
                self.hsl.l = (self.hsl.l + delta / 100.0).clamp(0.0, 1.0);
            }
        }
    }

    pub fn next_component(&mut self) {
        self.active = match self.active {
            HslComponent::H => HslComponent::S,
            HslComponent::S => HslComponent::L,
            HslComponent::L => HslComponent::H,
        };
    }

    pub fn prev_component(&mut self) {
        self.active = match self.active {
            HslComponent::H => HslComponent::L,
            HslComponent::S => HslComponent::H,
            HslComponent::L => HslComponent::S,
        };
    }
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                for nc in chars.by_ref() {
                    if nc.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn pad_to(s: &str, width: usize) -> String {
    let visible = strip_ansi(s).chars().count();
    if visible < width {
        format!("{}{}", s, " ".repeat(width - visible))
    } else {
        s.to_string()
    }
}

fn render_bar(value: f64, max: f64, width: usize, is_active: bool) -> String {
    let frac = (value / max).clamp(0.0, 1.0);
    let filled = (frac * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));
    if is_active {
        format!("\x1b[7m{}\x1b[0m", bar)
    } else {
        bar
    }
}

pub fn render_color_picker(picker: &ColorPickerState, cols: usize) -> Vec<String> {
    let cols = cols.max(20);
    let bar_width = cols.saturating_sub(12).max(4);

    let active_h = picker.active == HslComponent::H;
    let active_s = picker.active == HslComponent::S;
    let active_l = picker.active == HslComponent::L;

    let h_bar = render_bar(picker.hsl.h, 360.0, bar_width, active_h);
    let s_bar = render_bar(picker.hsl.s, 1.0, bar_width, active_s);
    let l_bar = render_bar(picker.hsl.l, 1.0, bar_width, active_l);

    let h_label = if active_h { "\x1b[1mH:\x1b[0m" } else { "H:" };
    let s_label = if active_s { "\x1b[1mS:\x1b[0m" } else { "S:" };
    let l_label = if active_l { "\x1b[1mL:\x1b[0m" } else { "L:" };

    let line_h = format!("{} {:3.0}° {}", h_label, picker.hsl.h, h_bar);
    let line_s = format!("{} {:3.0}% {}", s_label, picker.hsl.s * 100.0, s_bar);
    let line_l = format!("{} {:3.0}% {}", l_label, picker.hsl.l * 100.0, l_bar);

    let hex = picker.current_hex();
    let preview_bg = ansi_bg(&hex);
    let preview_fg = ansi_fg(&crate::colors::derive_text_color(&hex));
    let preview_inner = format!("  {}  ", hex);
    let line_preview = format!("{}{}{}\x1b[0m", preview_bg, preview_fg, preview_inner);

    let line_footer = "[↑↓] row  [←→] adjust  [↵] ok  [Esc] ✕".to_string();

    let mut lines = vec![line_h, line_s, line_l, line_preview, line_footer];

    for line in &mut lines {
        *line = pad_to(line, cols);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_no_initial_hex_uses_defaults() {
        let picker = ColorPickerState::new(0, None);
        assert!((picker.hsl.h - 180.0).abs() < 0.001);
        assert!((picker.hsl.s - 0.5).abs() < 0.001);
        assert!((picker.hsl.l - 0.5).abs() < 0.001);
        assert_eq!(picker.active, HslComponent::H);
        assert_eq!(picker.target_tab, 0);
    }

    #[test]
    fn test_new_with_hex_parses_hsl() {
        // pure red: hue≈0, saturation=1, lightness=0.5
        let picker = ColorPickerState::new(2, Some("#ff0000"));
        assert!(picker.hsl.h < 1.0 || picker.hsl.h > 359.0);
        assert!((picker.hsl.s - 1.0).abs() < 0.01);
        assert_eq!(picker.target_tab, 2);
    }

    #[test]
    fn test_new_invalid_hex_falls_back_to_defaults() {
        let picker = ColorPickerState::new(0, Some("not-a-color"));
        assert!((picker.hsl.h - 180.0).abs() < 0.001);
    }

    #[test]
    fn test_adjust_h_increases_hue() {
        let mut picker = ColorPickerState::new(0, None);
        picker.active = HslComponent::H;
        let before = picker.hsl.h;
        picker.adjust(5.0);
        assert!((picker.hsl.h - (before + 5.0)).abs() < 0.001);
    }

    #[test]
    fn test_adjust_h_wraps_around_360() {
        let mut picker = ColorPickerState::new(0, None);
        picker.active = HslComponent::H;
        picker.hsl.h = 358.0;
        picker.adjust(5.0);
        // 358 + 5 = 363 → rem_euclid(360) = 3
        assert!(picker.hsl.h < 10.0, "hue should wrap around 360");
    }

    #[test]
    fn test_adjust_h_negative_wraps() {
        let mut picker = ColorPickerState::new(0, None);
        picker.active = HslComponent::H;
        picker.hsl.h = 2.0;
        picker.adjust(-5.0);
        // -3 rem_euclid(360) = 357
        assert!(picker.hsl.h > 350.0, "negative hue should wrap to near 360");
    }

    #[test]
    fn test_adjust_s_decreases_saturation() {
        let mut picker = ColorPickerState::new(0, None);
        picker.active = HslComponent::S;
        picker.hsl.s = 0.5;
        picker.adjust(-5.0);
        assert!((picker.hsl.s - 0.45).abs() < 0.001);
    }

    #[test]
    fn test_adjust_s_clamps_at_zero() {
        let mut picker = ColorPickerState::new(0, None);
        picker.active = HslComponent::S;
        picker.hsl.s = 0.02;
        picker.adjust(-5.0);
        assert_eq!(picker.hsl.s, 0.0);
    }

    #[test]
    fn test_adjust_l_clamps_at_one() {
        let mut picker = ColorPickerState::new(0, None);
        picker.active = HslComponent::L;
        picker.hsl.l = 0.98;
        picker.adjust(5.0);
        assert_eq!(picker.hsl.l, 1.0);
    }

    #[test]
    fn test_adjust_l_decreases() {
        let mut picker = ColorPickerState::new(0, None);
        picker.active = HslComponent::L;
        picker.hsl.l = 0.5;
        picker.adjust(-5.0);
        assert!((picker.hsl.l - 0.45).abs() < 0.001);
    }

    #[test]
    fn test_current_hex_returns_valid_hex() {
        let picker = ColorPickerState::new(0, None);
        let hex = picker.current_hex();
        assert!(hex.starts_with('#'), "hex should start with #");
        assert_eq!(hex.len(), 7, "hex should be 7 chars (# + 6 hex digits)");
        assert!(
            hex[1..].chars().all(|c| c.is_ascii_hexdigit()),
            "hex digits should all be valid"
        );
    }

    #[test]
    fn test_next_component_cycles_h_s_l() {
        let mut picker = ColorPickerState::new(0, None);
        assert_eq!(picker.active, HslComponent::H);
        picker.next_component();
        assert_eq!(picker.active, HslComponent::S);
        picker.next_component();
        assert_eq!(picker.active, HslComponent::L);
        picker.next_component();
        assert_eq!(picker.active, HslComponent::H);
    }

    #[test]
    fn test_prev_component_cycles_h_l_s_h() {
        let mut picker = ColorPickerState::new(0, None);
        assert_eq!(picker.active, HslComponent::H);
        picker.prev_component();
        assert_eq!(picker.active, HslComponent::L);
        picker.prev_component();
        assert_eq!(picker.active, HslComponent::S);
        picker.prev_component();
        assert_eq!(picker.active, HslComponent::H);
    }

    #[test]
    fn test_render_returns_exactly_5_lines() {
        let picker = ColorPickerState::new(0, None);
        let lines = render_color_picker(&picker, 40);
        assert_eq!(lines.len(), 5, "render must produce exactly 5 lines");
    }

    #[test]
    fn test_render_h_line_contains_degree_symbol() {
        let picker = ColorPickerState::new(0, None);
        let lines = render_color_picker(&picker, 40);
        assert!(
            lines[0].contains('°'),
            "H line should contain degree symbol"
        );
    }

    #[test]
    fn test_render_s_line_contains_percent() {
        let picker = ColorPickerState::new(0, None);
        let lines = render_color_picker(&picker, 40);
        assert!(lines[1].contains('%'), "S line should contain %");
    }

    #[test]
    fn test_render_l_line_contains_percent() {
        let picker = ColorPickerState::new(0, None);
        let lines = render_color_picker(&picker, 40);
        assert!(lines[2].contains('%'), "L line should contain %");
    }

    #[test]
    fn test_render_preview_line_contains_hex() {
        let picker = ColorPickerState::new(0, None);
        let hex = picker.current_hex();
        let lines = render_color_picker(&picker, 40);
        assert!(
            lines[3].contains(&hex),
            "preview line should contain hex value"
        );
    }

    #[test]
    fn test_render_footer_contains_esc_hint() {
        let picker = ColorPickerState::new(0, None);
        let lines = render_color_picker(&picker, 40);
        assert!(lines[4].contains("Esc"), "footer should contain Esc hint");
    }

    #[test]
    fn test_render_active_h_line_has_highlight() {
        let picker = ColorPickerState::new(0, None); // active = H
        let lines = render_color_picker(&picker, 40);
        // Active component uses bold or reverse video
        assert!(
            lines[0].contains("\x1b["),
            "active H line should have ANSI escape"
        );
    }

    #[test]
    fn test_strip_ansi_removes_escape_codes() {
        let s = "\x1b[1mHello\x1b[0m";
        assert_eq!(strip_ansi(s), "Hello");
    }

    #[test]
    fn test_strip_ansi_preserves_plain_text() {
        let s = "Hello World";
        assert_eq!(strip_ansi(s), "Hello World");
    }

    #[test]
    fn test_render_lines_padded_to_cols() {
        let picker = ColorPickerState::new(0, None);
        let cols = 50;
        let lines = render_color_picker(&picker, cols);
        for (i, line) in lines.iter().enumerate() {
            let visible = strip_ansi(line).chars().count();
            assert!(
                visible >= cols,
                "line {} visible width {} should be >= cols {}",
                i,
                visible,
                cols
            );
        }
    }
}
