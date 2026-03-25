use crate::click::ClickRegion;
use crate::config::Config;
use crate::indicators::IndicatorState;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use zellij_tile::prelude::*;

/// Typed composite key for per-tab state maps.
/// Serializes/deserializes as `"name::position"` for backward compat with state.json.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TabKey {
    pub name: String,
    pub position: usize,
}

impl TabKey {
    pub fn new(name: &str, position: usize) -> Self {
        TabKey {
            name: name.to_string(),
            position,
        }
    }
}

impl fmt::Display for TabKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}::{}", self.name, self.position)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TabKeyParseError;

impl fmt::Display for TabKeyParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid TabKey format, expected 'name::position'")
    }
}

impl FromStr for TabKey {
    type Err = TabKeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Split on last "::" to allow tab names that contain "::"
        let sep = "::";
        let idx = s.rfind(sep).ok_or(TabKeyParseError)?;
        let name = &s[..idx];
        let pos_str = &s[idx + sep.len()..];
        let position = pos_str.parse::<usize>().map_err(|_| TabKeyParseError)?;
        Ok(TabKey {
            name: name.to_string(),
            position,
        })
    }
}

impl Serialize for TabKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for TabKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse::<TabKey>().map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Default)]
pub struct TabEntry {
    pub position: usize,
    pub name: String,
    pub active: bool,
    pub panes: Vec<PaneEntry>,
}

/// A processed pane entry.
/// is_plugin panes are tracked but filtered out of the rendered tree.
#[derive(Clone, Default)]
pub struct PaneEntry {
    pub id: u32,
    pub is_plugin: bool,
    pub title: String, // running command or locked name
    pub is_focused: bool,
    #[allow(dead_code)]
    pub is_floating: bool,
    #[allow(dead_code)]
    pub cwd: Option<std::path::PathBuf>,
}

#[derive(Default)]
pub struct PluginState {
    pub config: Config,
    pub plugin_id: u32,
    pub rows: usize,
    pub cols: usize,
    pub tabs: Vec<TabInfo>,
    pub pane_manifest: Option<PaneManifest>,
    pub active_tab_index: usize,
    pub tab_entries: Vec<TabEntry>,
    pub group_assignments: HashMap<String, String>,
    pub collapsed_groups: HashSet<String>,
    pub viewport_offset: usize,
    pub max_viewport_offset: usize,
    pub sidebar_collapsed: bool,
    pub cursor_position: Option<usize>,
    pub active_menu: Option<MenuState>,
    pub rename_state: Option<RenameState>,
    pub indicators: HashMap<String, IndicatorState>,
    pub custom_colors: HashMap<String, String>,
    pub markers: HashMap<String, String>,
    pub tick_count: u64,
    #[allow(dead_code)]
    pub last_save_tick: u64,
    pub git_status: Option<GitStatus>,
    pub pane_cwds: HashMap<u32, PathBuf>,
    pub click_regions: Vec<ClickRegion>,
}

#[derive(Default, Clone)]
pub struct MenuState {
    pub target: MenuTarget,
    pub selected_index: usize,
    pub position_line: usize,
}

#[derive(Default, Clone)]
pub enum MenuTarget {
    #[default]
    None,
    Tab(usize),
    Pane(u32),
    Group(String),
}

#[derive(Default, Clone)]
pub struct RenameState {
    pub target: RenameTarget,
    pub buffer: String,
}

#[derive(Default, Clone)]
pub enum RenameTarget {
    #[default]
    None,
    Tab(usize),
    Pane(u32),
    Group(String),
}

#[derive(Default, Clone)]
pub struct GitStatus {
    pub branch: String,
    pub dirty: usize,
    pub staged: usize,
    pub ahead: usize,
    pub behind: usize,
}

impl PluginState {
    pub fn load_config(&mut self, _configuration: BTreeMap<String, String>) {
        self.config = Config::load();
    }

    pub fn load_persisted_state(&mut self) {
        let state = crate::persistence::load_state();
        self.group_assignments = state.group_assignments;
        self.collapsed_groups = state.collapsed_groups;
        self.custom_colors = state.custom_colors;
        self.markers = state.markers;
        self.sidebar_collapsed = state.sidebar_collapsed;
    }

    #[allow(dead_code)]
    pub fn tab_key(&self, tab: &TabInfo) -> String {
        format!("{}::{}", tab.name, tab.position)
    }

    pub fn rebuild_tab_entries(&mut self) {
        self.tab_entries = self
            .tabs
            .iter()
            .map(|tab| {
                let panes = self
                    .pane_manifest
                    .as_ref()
                    .and_then(|m| m.panes.get(&tab.position))
                    .map(|pane_list| {
                        pane_list
                            .iter()
                            .map(|p| PaneEntry {
                                id: p.id,
                                is_plugin: p.is_plugin,
                                title: p.title.clone(),
                                is_focused: p.is_focused,
                                is_floating: p.is_floating,
                                cwd: self.pane_cwds.get(&p.id).cloned(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                TabEntry {
                    position: tab.position,
                    name: tab.name.clone(),
                    active: tab.active,
                    panes,
                }
            })
            .collect();
    }

    #[allow(dead_code)]
    pub fn terminal_panes_for_tab(&self, tab_position: usize) -> Vec<&PaneEntry> {
        self.tab_entries
            .iter()
            .find(|t| t.position == tab_position)
            .map(|t| t.panes.iter().filter(|p| !p.is_plugin).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tab(position: usize, name: &str, active: bool) -> TabInfo {
        let mut tab = TabInfo::default();
        tab.position = position;
        tab.name = name.to_string();
        tab.active = active;
        tab
    }

    #[test]
    fn test_tab_key_format() {
        let state = PluginState::default();
        let tab = make_tab(0, "api", true);
        assert_eq!(state.tab_key(&tab), "api::0");
    }

    #[test]
    fn test_tab_key_unique_positions() {
        let state = PluginState::default();
        let tab1 = make_tab(0, "api", true);
        let tab2 = make_tab(1, "api", false);
        assert_ne!(state.tab_key(&tab1), state.tab_key(&tab2));
    }

    #[test]
    fn test_rebuild_no_tabs() {
        let mut state = PluginState::default();
        state.tabs = vec![];
        state.rebuild_tab_entries();
        assert!(state.tab_entries.is_empty());
    }

    #[test]
    fn test_rebuild_three_tabs() {
        let mut state = PluginState::default();
        state.tabs = vec![
            make_tab(0, "dashboard", false),
            make_tab(1, "api", true),
            make_tab(2, "tests", false),
        ];
        state.rebuild_tab_entries();
        assert_eq!(state.tab_entries.len(), 3);
        assert_eq!(state.tab_entries[0].name, "dashboard");
        assert!(!state.tab_entries[0].active);
        assert_eq!(state.tab_entries[1].name, "api");
        assert!(state.tab_entries[1].active);
    }

    #[test]
    fn test_active_tab_index_tracking() {
        let mut state = PluginState::default();
        state.tabs = vec![
            make_tab(0, "tab1", false),
            make_tab(1, "tab2", false),
            make_tab(2, "tab3", true),
        ];
        state.active_tab_index = state.tabs.iter().position(|t| t.active).unwrap_or(0);
        assert_eq!(state.active_tab_index, 2);
    }

    #[test]
    fn test_terminal_panes_empty_when_no_manifest() {
        let state = PluginState::default();
        let panes = state.terminal_panes_for_tab(0);
        assert!(panes.is_empty());
    }
}
