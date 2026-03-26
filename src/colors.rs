pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}
pub struct Hsl {
    pub h: f64,
    pub s: f64,
    pub l: f64,
}

pub fn hex_to_rgb(hex: &str) -> Option<Rgb> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    Some(Rgb {
        r: u8::from_str_radix(&hex[0..2], 16).ok()?,
        g: u8::from_str_radix(&hex[2..4], 16).ok()?,
        b: u8::from_str_radix(&hex[4..6], 16).ok()?,
    })
}

pub fn rgb_to_hex(rgb: &Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b)
}

/// h: 0-360, s: 0-1, l: 0-1
pub fn hex_to_hsl(hex: &str) -> Option<Hsl> {
    let rgb = hex_to_rgb(hex)?;
    Some(rgb_to_hsl(&rgb))
}

pub fn rgb_to_hsl(rgb: &Rgb) -> Hsl {
    let rf = rgb.r as f64 / 255.0;
    let gf = rgb.g as f64 / 255.0;
    let bf = rgb.b as f64 / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let l = (max + min) / 2.0;

    if (max - min).abs() < f64::EPSILON {
        return Hsl { h: 0.0, s: 0.0, l };
    }

    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };

    let h = if (max - rf).abs() < f64::EPSILON {
        let mut h = (gf - bf) / d;
        if gf < bf {
            h += 6.0;
        }
        h * 60.0
    } else if (max - gf).abs() < f64::EPSILON {
        ((bf - rf) / d + 2.0) * 60.0
    } else {
        ((rf - gf) / d + 4.0) * 60.0
    };

    Hsl { h, s, l }
}

pub fn hsl_to_rgb(hsl: &Hsl) -> Rgb {
    if hsl.s.abs() < f64::EPSILON {
        let v = (hsl.l * 255.0) as u8;
        return Rgb { r: v, g: v, b: v };
    }

    let q = if hsl.l < 0.5 {
        hsl.l * (1.0 + hsl.s)
    } else {
        hsl.l + hsl.s - hsl.l * hsl.s
    };
    let p = 2.0 * hsl.l - q;

    let r = hue_to_rgb(p, q, hsl.h / 360.0 + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, hsl.h / 360.0);
    let b = hue_to_rgb(p, q, hsl.h / 360.0 - 1.0 / 3.0);

    Rgb {
        r: (r * 255.0).round() as u8,
        g: (g * 255.0).round() as u8,
        b: (b * 255.0).round() as u8,
    }
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 0.5 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

pub fn hsl_to_hex(hsl: &Hsl) -> String {
    rgb_to_hex(&hsl_to_rgb(hsl))
}

/// WCAG relative luminance: 0=black, 1=white. Formula: 0.2126R + 0.7152G + 0.0722B (linearised).
pub fn get_luminance(hex: &str) -> f64 {
    let rgb = match hex_to_rgb(hex) {
        Some(r) => r,
        None => return 0.0,
    };
    let gamma = |v: f64| -> f64 {
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let r = gamma(rgb.r as f64 / 255.0);
    let g = gamma(rgb.g as f64 / 255.0);
    let b = gamma(rgb.b as f64 / 255.0);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// WCAG contrast ratio: range 1-21. (lighter + 0.05) / (darker + 0.05).
pub fn get_contrast_ratio(c1: &str, c2: &str) -> f64 {
    let l1 = get_luminance(c1);
    let l2 = get_luminance(c2);
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

pub fn is_light_color(hex: &str) -> bool {
    get_luminance(hex) > 0.5
}

/// Returns "#ffffff" or "#000000" — whichever achieves WCAG AA ratio >= 3 on bg_hex.
pub fn derive_text_color(bg_hex: &str) -> String {
    if get_contrast_ratio("#ffffff", bg_hex) >= 3.0 {
        "#ffffff".into()
    } else if is_light_color(bg_hex) {
        "#000000".into()
    } else {
        "#ffffff".into()
    }
}

pub fn derive_active_bg(base_hex: &str, is_dark: bool) -> String {
    let mut hsl = match hex_to_hsl(base_hex) {
        Some(h) => h,
        None => return base_hex.into(),
    };
    hsl.s = (hsl.s * 1.4).min(1.0);
    if is_dark {
        hsl.l = (hsl.l * 1.2).clamp(0.35, 0.6);
    } else {
        hsl.l = (hsl.l * 0.9).clamp(0.25, 0.5);
    }
    hsl_to_hex(&hsl)
}

pub fn derive_inactive_bg(base_hex: &str, is_dark: bool) -> String {
    let mut hsl = match hex_to_hsl(base_hex) {
        Some(h) => h,
        None => return base_hex.into(),
    };
    hsl.s *= 0.7;
    if is_dark {
        hsl.l = (hsl.l * 1.1).min(0.45);
    } else {
        hsl.l = (hsl.l * 0.95).min(0.4);
    }
    hsl_to_hex(&hsl)
}

/// Returns (bg, fg, active_bg, active_fg, inactive_bg, inactive_fg) — all hex strings.
pub fn derive_theme_colors(
    base_hex: &str,
    is_dark: bool,
) -> (String, String, String, String, String, String) {
    let bg = base_hex.to_string();
    let active_bg = derive_active_bg(base_hex, is_dark);
    let inactive_bg = derive_inactive_bg(base_hex, is_dark);
    let fg = derive_text_color(&bg);
    let active_fg = derive_text_color(&active_bg);
    let inactive_fg = derive_text_color(&inactive_bg);
    (bg, fg, active_bg, active_fg, inactive_bg, inactive_fg)
}

pub fn get_default_group_color(index: usize) -> &'static str {
    const PALETTE: &[&str] = &[
        "#3498db", "#2ecc71", "#e74c3c", "#9b59b6", "#f39c12", "#1abc9c", "#e67e22", "#34495e",
        "#16a085", "#c0392b", "#8e44ad", "#27ae60",
    ];
    PALETTE[index % PALETTE.len()]
}

pub fn ansi_fg(hex: &str) -> String {
    if let Some(rgb) = hex_to_rgb(hex) {
        format!("\x1b[38;2;{};{};{}m", rgb.r, rgb.g, rgb.b)
    } else {
        String::new()
    }
}

pub fn ansi_bg(hex: &str) -> String {
    if let Some(rgb) = hex_to_rgb(hex) {
        format!("\x1b[48;2;{};{};{}m", rgb.r, rgb.g, rgb.b)
    } else {
        String::new()
    }
}

pub const RESET: &str = "\x1b[0m";
pub const REVERSE: &str = "\x1b[7m";

#[derive(Clone, Debug)]
pub struct SidebarTheme {
    pub sidebar_bg: String,
    pub sidebar_fg: String,
    pub divider_fg: String,
    pub is_dark: bool,
    pub menu_selected_bg: String,
    pub menu_selected_fg: String,
    pub menu_bg: String,
    pub menu_fg: String,
}

pub fn catppuccin_mocha() -> SidebarTheme {
    SidebarTheme {
        sidebar_bg: "#1e1e2e".into(),
        sidebar_fg: "#cdd6f4".into(),
        divider_fg: "#45475a".into(),
        is_dark: true,
        menu_selected_bg: "#3c3c50".into(),
        menu_selected_fg: "#ffffff".into(),
        menu_bg: "#23232d".into(),
        menu_fg: "#c8c8c8".into(),
    }
}

pub fn rose_pine_dawn() -> SidebarTheme {
    SidebarTheme {
        sidebar_bg: "#faf4ed".into(),
        sidebar_fg: "#575279".into(),
        divider_fg: "#dfdad9".into(),
        is_dark: false,
        menu_selected_bg: "#cecacd".into(),
        menu_selected_fg: "#575279".into(),
        menu_bg: "#ebe5e0".into(),
        menu_fg: "#575279".into(),
    }
}

pub fn get_theme(name: &str) -> SidebarTheme {
    match name {
        "rose-pine-dawn" => rose_pine_dawn(),
        "catppuccin-mocha" => catppuccin_mocha(),
        other => {
            if !other.is_empty() {
                eprintln!(
                    "tabby-zj: unknown theme '{}', using catppuccin-mocha",
                    other
                );
            }
            catppuccin_mocha()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_rgb_valid() {
        let rgb = hex_to_rgb("#e74c3c").unwrap();
        assert_eq!(rgb.r, 231);
        assert_eq!(rgb.g, 76);
        assert_eq!(rgb.b, 60);
    }

    #[test]
    fn test_hex_to_rgb_invalid() {
        assert!(hex_to_rgb("nothex").is_none());
        assert!(hex_to_rgb("#gg0000").is_none());
    }

    #[test]
    fn test_rgb_to_hex_roundtrip() {
        let original = "#3498db";
        let rgb = hex_to_rgb(original).unwrap();
        assert_eq!(rgb_to_hex(&rgb), original);
    }

    #[test]
    fn test_rgb_hsl_roundtrip() {
        let rgb = hex_to_rgb("#e74c3c").unwrap();
        let hsl = rgb_to_hsl(&rgb);
        assert!(hsl.s > 0.5, "red should be highly saturated");
        let back = hsl_to_rgb(&hsl);
        assert!(
            (rgb.r as i16 - back.r as i16).abs() <= 2,
            "r roundtrip: {} vs {}",
            rgb.r,
            back.r
        );
        assert!(
            (rgb.g as i16 - back.g as i16).abs() <= 2,
            "g roundtrip: {} vs {}",
            rgb.g,
            back.g
        );
        assert!(
            (rgb.b as i16 - back.b as i16).abs() <= 2,
            "b roundtrip: {} vs {}",
            rgb.b,
            back.b
        );
    }

    #[test]
    fn test_gray_has_zero_saturation() {
        let rgb = hex_to_rgb("#808080").unwrap();
        let hsl = rgb_to_hsl(&rgb);
        assert!(hsl.s < 0.01, "gray should have near-zero saturation");
    }

    #[test]
    fn test_derive_text_color_on_dark() {
        let fg = derive_text_color("#1a1b26");
        assert_eq!(fg, "#ffffff", "should use white on dark bg");
    }

    #[test]
    fn test_derive_text_color_on_light() {
        let fg = derive_text_color("#fafafa");
        assert_eq!(fg, "#000000", "should use black on light bg");
    }

    #[test]
    fn test_derive_active_bg_increases_vibrancy() {
        let base = "#3498db";
        let active = derive_active_bg(base, true);
        let base_hsl = hex_to_hsl(base).unwrap();
        let active_hsl = hex_to_hsl(&active).unwrap();
        assert!(
            active_hsl.s >= base_hsl.s,
            "active should be at least as saturated as base: {} vs {}",
            active_hsl.s,
            base_hsl.s
        );
    }

    #[test]
    fn test_derive_theme_colors_returns_six_strings() {
        let (bg, fg, abg, afg, ibg, ifg) = derive_theme_colors("#e74c3c", true);
        assert!(bg.starts_with('#'));
        assert!(fg.starts_with('#'));
        assert!(abg.starts_with('#'));
        assert!(afg.starts_with('#'));
        assert!(ibg.starts_with('#'));
        assert!(ifg.starts_with('#'));
    }

    #[test]
    fn test_default_group_color_palette() {
        assert_eq!(get_default_group_color(0), "#3498db");
        assert_eq!(get_default_group_color(12), "#3498db");
        assert_ne!(get_default_group_color(0), get_default_group_color(1));
    }

    #[test]
    fn test_get_contrast_ratio_black_white() {
        let ratio = get_contrast_ratio("#000000", "#ffffff");
        assert!(
            (ratio - 21.0).abs() < 0.1,
            "black/white should be ~21:1 contrast, got {}",
            ratio
        );
    }

    #[test]
    fn test_catppuccin_mocha_is_dark() {
        assert!(
            catppuccin_mocha().is_dark,
            "catppuccin-mocha should be dark"
        );
    }

    #[test]
    fn test_rose_pine_dawn_is_light() {
        assert!(!rose_pine_dawn().is_dark, "rose-pine-dawn should be light");
    }

    #[test]
    fn test_get_theme_mocha() {
        assert!(
            get_theme("catppuccin-mocha").is_dark,
            "get_theme catppuccin-mocha should be dark"
        );
    }

    #[test]
    fn test_get_theme_dawn() {
        assert!(
            !get_theme("rose-pine-dawn").is_dark,
            "get_theme rose-pine-dawn should be light"
        );
    }

    #[test]
    fn test_get_theme_unknown_falls_back_to_dark() {
        assert!(
            get_theme("unknown-theme").is_dark,
            "unknown theme should fall back to catppuccin-mocha (dark)"
        );
    }

    #[test]
    fn test_get_theme_empty_falls_back_to_dark() {
        assert!(
            get_theme("").is_dark,
            "empty theme name should fall back to catppuccin-mocha (dark)"
        );
    }

    #[test]
    fn test_sidebar_bg_is_valid_hex() {
        let t = catppuccin_mocha();
        assert!(
            t.sidebar_bg.starts_with('#') && t.sidebar_bg.len() == 7,
            "catppuccin-mocha sidebar_bg should be a 7-char hex: {}",
            t.sidebar_bg
        );
    }

    #[test]
    fn test_rose_pine_sidebar_bg_is_valid_hex() {
        let t = rose_pine_dawn();
        assert!(
            t.sidebar_bg.starts_with('#') && t.sidebar_bg.len() == 7,
            "rose-pine-dawn sidebar_bg should be a 7-char hex: {}",
            t.sidebar_bg
        );
    }
}
