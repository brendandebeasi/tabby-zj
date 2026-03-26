use serde::{Deserialize, Serialize};
use serde_yaml_ng as serde_yaml;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub groups: Vec<GroupConfig>,
    #[serde(default)]
    pub sidebar: SidebarConfig,
    #[serde(default)]
    pub indicators: IndicatorConfig,
    #[serde(default)]
    pub widgets: WidgetConfig,
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    #[serde(default)]
    pub cursor_up: Option<String>,
    #[serde(default)]
    pub cursor_down: Option<String>,
    #[serde(default)]
    pub activate: Option<String>,
    #[serde(default)]
    pub dismiss: Option<String>,
    #[serde(default)]
    pub toggle_collapse: Option<String>,
    #[serde(default)]
    pub new_tab: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupConfig {
    pub name: String,
    #[serde(default)]
    pub pattern: String,
    #[serde(default)]
    pub working_dir: Option<String>,
    #[serde(default)]
    pub theme: ThemeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_bg")]
    pub bg: String,
    #[serde(default = "default_fg")]
    pub fg: String,
    #[serde(default)]
    pub active_bg: Option<String>,
    #[serde(default)]
    pub active_fg: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

fn default_bg() -> String {
    "#3c3836".into()
}
fn default_fg() -> String {
    "#ebdbb2".into()
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            bg: default_bg(),
            fg: default_fg(),
            active_bg: None,
            active_fg: None,
            icon: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarConfig {
    #[serde(default = "default_width")]
    pub width: usize,
    #[serde(default = "default_theme_name")]
    pub theme: String,
    #[serde(default = "default_sort")]
    pub sort_by: String,
    #[serde(default = "default_show_panes")]
    pub show_panes: bool,
    #[serde(default)]
    pub show_empty_groups: bool,
}

fn default_width() -> usize {
    25
}
fn default_theme_name() -> String {
    "catppuccin-mocha".into()
}
fn default_sort() -> String {
    "group".into()
}
fn default_show_panes() -> bool {
    true
}

impl Default for SidebarConfig {
    fn default() -> Self {
        Self {
            width: default_width(),
            theme: default_theme_name(),
            sort_by: default_sort(),
            show_panes: default_show_panes(),
            show_empty_groups: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndicatorConfig {
    #[serde(default)]
    pub busy: IndicatorDef,
    #[serde(default)]
    pub bell: IndicatorDef,
    #[serde(default)]
    pub input: IndicatorDef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndicatorDef {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for IndicatorDef {
    fn default() -> Self {
        Self {
            enabled: true,
            icon: None,
            color: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WidgetConfig {
    #[serde(default)]
    pub clock: ClockWidgetConfig,
    #[serde(default)]
    pub git: GitWidgetConfig,
    #[serde(default)]
    pub stats: StatsWidgetConfig,
    #[serde(default)]
    pub quota: QuotaWidgetConfig,
    #[serde(default)]
    pub pet: PetWidgetConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockWidgetConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_time_fmt")]
    pub format: String,
    #[serde(default = "default_true")]
    pub show_date: bool,
}

fn default_time_fmt() -> String {
    "%H:%M:%S".into()
}

impl Default for ClockWidgetConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            format: default_time_fmt(),
            show_date: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitWidgetConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_git_interval")]
    pub interval_secs: u64,
}

fn default_git_interval() -> u64 {
    5
}

impl Default for GitWidgetConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: default_git_interval(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsWidgetConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_stats_interval")]
    pub interval_secs: u64,
}

fn default_stats_interval() -> u64 {
    30
}

impl Default for StatsWidgetConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_secs: default_stats_interval(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuotaWidgetConfig {
    #[serde(default)]
    pub enabled: bool,
}

fn default_pet_name() -> String {
    "Whiskers".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PetWidgetConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_pet_name")]
    pub name: String,
}

impl Default for PetWidgetConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            name: default_pet_name(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_dir = std::env::var("TABBY_ZJ_CONFIG_DIR").unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
            format!("{}/.config/tabby-zj", home)
        });
        let config_path = format!("{}/config.yaml", config_dir);

        match std::fs::read_to_string(&config_path) {
            Ok(contents) => match serde_yaml::from_str::<Config>(&contents) {
                Ok(config) => config,
                Err(e) => {
                    eprintln!("tabby-zj: config parse error in {}: {}", config_path, e);
                    Config::default()
                }
            },
            Err(_) => {
                // File doesn't exist — use defaults silently
                Config::default()
            }
        }
    }

    /// Parse config from a YAML string (for testing)
    #[allow(dead_code)]
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.groups.len(), 0);
        assert_eq!(config.sidebar.width, 25);
        assert_eq!(config.sidebar.theme, "catppuccin-mocha");
        assert!(config.widgets.clock.enabled);
        assert!(config.indicators.busy.enabled);
    }

    #[test]
    fn test_parse_three_groups() {
        let yaml = r##"
groups:
  - name: Frontend
    pattern: "^FE\\|"
    theme:
      bg: "#e74c3c"
      fg: "#ffffff"
  - name: Backend
    pattern: "^BE\\|"
    theme:
      bg: "#3498db"
      fg: "#ffffff"
  - name: Default
    pattern: ".*"
    theme:
      bg: "#3c3836"
      fg: "#ebdbb2"
"##;
        let config = Config::from_yaml(yaml).expect("should parse");
        assert_eq!(config.groups.len(), 3);
        assert_eq!(config.groups[0].name, "Frontend");
        assert_eq!(config.groups[0].pattern, "^FE\\|");
        assert_eq!(config.groups[0].theme.bg, "#e74c3c");
        assert_eq!(config.groups[1].name, "Backend");
        assert_eq!(config.groups[2].name, "Default");
    }

    #[test]
    fn test_invalid_yaml_fallback() {
        let result = Config::from_yaml("{{invalid: yaml: [");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_yaml() {
        let config = Config::from_yaml("").expect("empty yaml should parse to defaults");
        assert_eq!(config.groups.len(), 0);
        assert_eq!(config.sidebar.width, 25);
    }

    #[test]
    fn test_partial_config_missing_widgets() {
        let yaml = r#"
groups:
  - name: Default
    pattern: ".*"
"#;
        let config = Config::from_yaml(yaml).expect("partial config should parse");
        assert_eq!(config.groups.len(), 1);
        assert!(config.widgets.clock.enabled);
        assert_eq!(config.widgets.clock.format, "%H:%M:%S");
        assert!(config.widgets.git.enabled);
        assert!(!config.widgets.stats.enabled);
        assert_eq!(config.widgets.stats.interval_secs, 30);
    }

    #[test]
    fn test_stats_widget_defaults() {
        let config = Config::default();
        assert!(!config.widgets.stats.enabled);
        assert_eq!(config.widgets.stats.interval_secs, 30);
    }

    #[test]
    fn test_stats_widget_parses_from_yaml() {
        let yaml = r#"
widgets:
  stats:
    enabled: true
    interval_secs: 7
"#;
        let config = Config::from_yaml(yaml).expect("should parse");
        assert!(config.widgets.stats.enabled);
        assert_eq!(config.widgets.stats.interval_secs, 7);
    }

    #[test]
    fn test_theme_config_defaults() {
        let yaml = r#"
groups:
  - name: Test
    pattern: "test"
"#;
        let config = Config::from_yaml(yaml).expect("should parse");
        let theme = &config.groups[0].theme;
        assert_eq!(theme.bg, "#3c3836");
        assert_eq!(theme.fg, "#ebdbb2");
        assert!(theme.active_bg.is_none());
        assert!(theme.icon.is_none());
    }

    #[test]
    fn test_sidebar_config() {
        let yaml = r#"
sidebar:
  width: 30
  theme: rose-pine-dawn
  show_panes: true
"#;
        let config = Config::from_yaml(yaml).expect("should parse");
        assert_eq!(config.sidebar.width, 30);
        assert_eq!(config.sidebar.theme, "rose-pine-dawn");
        assert!(config.sidebar.show_panes);
        assert_eq!(config.sidebar.sort_by, "group");
    }

    #[test]
    fn test_indicator_config() {
        let yaml = r#"
indicators:
  busy:
    enabled: false
    icon: "●"
  bell:
    enabled: true
"#;
        let config = Config::from_yaml(yaml).expect("should parse");
        assert!(!config.indicators.busy.enabled);
        assert_eq!(config.indicators.busy.icon, Some("●".into()));
        assert!(config.indicators.bell.enabled);
        assert!(config.indicators.input.enabled);
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let prev = std::env::var("TABBY_ZJ_CONFIG_DIR").ok();
        std::env::set_var(
            "TABBY_ZJ_CONFIG_DIR",
            "/tmp/tabby-zj-nonexistent-test-dir-abc123",
        );
        let config = Config::load();
        match prev {
            Some(v) => std::env::set_var("TABBY_ZJ_CONFIG_DIR", v),
            None => std::env::remove_var("TABBY_ZJ_CONFIG_DIR"),
        }
        assert_eq!(config.groups.len(), 0);
        assert_eq!(config.sidebar.width, 25);
    }

    #[test]
    fn test_keybindings_defaults_are_none() {
        let config = Config::default();
        assert!(config.keybindings.cursor_up.is_none());
        assert!(config.keybindings.cursor_down.is_none());
        assert!(config.keybindings.activate.is_none());
        assert!(config.keybindings.dismiss.is_none());
        assert!(config.keybindings.toggle_collapse.is_none());
        assert!(config.keybindings.new_tab.is_none());
    }

    #[test]
    fn test_keybindings_parses_from_yaml() {
        let yaml = r#"
keybindings:
  cursor_down: "n"
  cursor_up: "p"
  activate: "Enter"
  dismiss: "Esc"
  toggle_collapse: "t"
  new_tab: "T"
"#;
        let config = Config::from_yaml(yaml).expect("should parse");
        assert_eq!(config.keybindings.cursor_down, Some("n".into()));
        assert_eq!(config.keybindings.cursor_up, Some("p".into()));
        assert_eq!(config.keybindings.activate, Some("Enter".into()));
        assert_eq!(config.keybindings.dismiss, Some("Esc".into()));
        assert_eq!(config.keybindings.toggle_collapse, Some("t".into()));
        assert_eq!(config.keybindings.new_tab, Some("T".into()));
    }

    #[test]
    fn test_keybindings_partial_override() {
        let yaml = r#"
keybindings:
  cursor_down: "n"
"#;
        let config = Config::from_yaml(yaml).expect("should parse");
        assert_eq!(config.keybindings.cursor_down, Some("n".into()));
        assert!(config.keybindings.cursor_up.is_none());
        assert!(config.keybindings.activate.is_none());
    }

    #[test]
    fn test_load_from_env_dir() {
        use std::io::Write;
        let prev = std::env::var("TABBY_ZJ_CONFIG_DIR").ok();
        let dir = std::env::temp_dir().join("tabby-zj-test-load-env");
        std::fs::create_dir_all(&dir).expect("create dir");
        let config_path = dir.join("config.yaml");
        let mut f = std::fs::File::create(&config_path).expect("create");
        write!(f, "sidebar:\n  width: 42\n").expect("write");

        std::env::set_var("TABBY_ZJ_CONFIG_DIR", dir.to_str().unwrap());
        let config = Config::load();
        let _ = std::fs::remove_dir_all(&dir);
        match prev {
            Some(v) => std::env::set_var("TABBY_ZJ_CONFIG_DIR", v),
            None => std::env::remove_var("TABBY_ZJ_CONFIG_DIR"),
        }
        assert_eq!(config.sidebar.width, 42);
    }

    #[test]
    fn test_quota_widget_defaults() {
        let config = Config::default();
        assert!(!config.widgets.quota.enabled);
    }

    #[test]
    fn test_quota_widget_parses_from_yaml() {
        let yaml = r#"
widgets:
  quota:
    enabled: true
"#;
        let config = Config::from_yaml(yaml).expect("should parse");
        assert!(config.widgets.quota.enabled);
    }

    #[test]
    fn test_pet_widget_defaults() {
        let config = Config::default();
        assert!(!config.widgets.pet.enabled);
        assert_eq!(config.widgets.pet.name, "Whiskers");
    }

    #[test]
    fn test_pet_widget_parses_from_yaml() {
        let yaml = r#"
widgets:
  pet:
    enabled: true
    name: "Luna"
"#;
        let config = Config::from_yaml(yaml).expect("should parse");
        assert!(config.widgets.pet.enabled);
        assert_eq!(config.widgets.pet.name, "Luna");
    }

    #[test]
    fn test_pet_widget_partial_defaults_name() {
        let yaml = r#"
widgets:
  pet:
    enabled: true
"#;
        let config = Config::from_yaml(yaml).expect("should parse");
        assert!(config.widgets.pet.enabled);
        assert_eq!(config.widgets.pet.name, "Whiskers");
    }
}
