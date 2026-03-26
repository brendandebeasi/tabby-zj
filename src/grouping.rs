use crate::colors;
use crate::config::{GroupConfig, ThemeConfig};
use crate::state::{TabEntry, TabKey};
use regex::Regex;
use std::collections::{HashMap, HashSet};

pub struct GroupedTabs {
    pub group_name: String,
    pub theme: ThemeConfig,
    pub tabs: Vec<TabEntry>,
    pub collapsed: bool,
}

pub fn assign_groups(
    tabs: &[TabEntry],
    group_configs: &[GroupConfig],
    manual: &HashMap<TabKey, String>,
    collapsed: &HashSet<String>,
    show_empty: bool,
    _sort_by: &str,
) -> Vec<GroupedTabs> {
    let compiled: Vec<(&GroupConfig, Option<Regex>)> = group_configs
        .iter()
        .map(|g| {
            let re = if g.pattern.is_empty() {
                None
            } else {
                Regex::new(&g.pattern).ok()
            };
            (g, re)
        })
        .collect();

    let mut group_buckets: HashMap<String, Vec<TabEntry>> = HashMap::new();
    for gc in group_configs {
        group_buckets.insert(gc.name.clone(), vec![]);
    }
    group_buckets.entry("Default".into()).or_default();

    for tab in tabs {
        let tab_key = TabKey::new(&tab.name, tab.position);

        if let Some(group_name) = manual.get(&tab_key) {
            group_buckets
                .entry(group_name.clone())
                .or_default()
                .push(tab.clone());
            continue;
        }

        let mut matched = false;
        for (gc, re) in &compiled {
            if let Some(re) = re {
                if re.is_match(&tab.name) {
                    group_buckets
                        .entry(gc.name.clone())
                        .or_default()
                        .push(tab.clone());
                    matched = true;
                    break;
                }
            }
        }

        if !matched {
            group_buckets
                .entry("Default".into())
                .or_default()
                .push(tab.clone());
        }
    }

    let mut result: Vec<GroupedTabs> = vec![];

    if let Some(tabs) = group_buckets.remove("Default") {
        if !tabs.is_empty() || show_empty {
            result.push(GroupedTabs {
                group_name: "Default".into(),
                theme: ThemeConfig::default(),
                tabs,
                collapsed: collapsed.contains("Default"),
            });
        }
    }

    let mut named: Vec<GroupedTabs> = group_configs
        .iter()
        .filter(|gc| gc.name != "Default")
        .filter_map(|gc| {
            let tabs = group_buckets.remove(&gc.name).unwrap_or_default();
            if tabs.is_empty() && !show_empty {
                return None;
            }
            Some(GroupedTabs {
                group_name: gc.name.clone(),
                theme: gc.theme.clone(),
                tabs,
                collapsed: collapsed.contains(&gc.name),
            })
        })
        .collect();
    named.sort_by(|a, b| a.group_name.cmp(&b.group_name));
    result.extend(named);

    result
}

pub fn auto_fill_theme(theme: &ThemeConfig, group_index: usize, is_dark: bool) -> ThemeConfig {
    let base = if theme.bg.is_empty() {
        colors::get_default_group_color(group_index).to_string()
    } else {
        theme.bg.clone()
    };

    let (bg, fg, active_bg, active_fg, _inactive_bg, _inactive_fg) =
        colors::derive_theme_colors(&base, is_dark);

    ThemeConfig {
        bg,
        fg: if theme.fg.is_empty() {
            fg
        } else {
            theme.fg.clone()
        },
        active_bg: Some(theme.active_bg.clone().unwrap_or(active_bg)),
        active_fg: Some(theme.active_fg.clone().unwrap_or(active_fg)),
        icon: theme.icon.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GroupConfig;
    use crate::state::TabEntry;

    fn make_tab(name: &str, position: usize) -> TabEntry {
        TabEntry {
            position,
            name: name.to_string(),
            active: false,
            panes: vec![],
        }
    }

    fn make_group(name: &str, pattern: &str) -> GroupConfig {
        GroupConfig {
            name: name.to_string(),
            pattern: pattern.to_string(),
            working_dir: None,
            theme: crate::config::ThemeConfig::default(),
        }
    }

    #[test]
    fn test_pattern_match_assigns_tab() {
        let tabs = vec![make_tab("FE|dashboard", 0)];
        let groups = vec![make_group("Frontend", r"^FE\|")];
        let result = assign_groups(
            &tabs,
            &groups,
            &HashMap::new(),
            &HashSet::new(),
            false,
            "index",
        );
        let frontend = result.iter().find(|g| g.group_name == "Frontend");
        assert!(frontend.is_some(), "Frontend group should exist");
        assert_eq!(frontend.unwrap().tabs.len(), 1);
    }

    #[test]
    fn test_manual_override_beats_pattern() {
        let tabs = vec![make_tab("FE|dashboard", 0)];
        let groups = vec![
            make_group("Frontend", r"^FE\|"),
            make_group("Backend", r"^BE\|"),
        ];
        let mut manual = HashMap::new();
        manual.insert(TabKey::new("FE|dashboard", 0), "Backend".to_string());
        let result = assign_groups(&tabs, &groups, &manual, &HashSet::new(), true, "index");
        let backend = result.iter().find(|g| g.group_name == "Backend");
        assert_eq!(backend.unwrap().tabs.len(), 1);
        let frontend = result.iter().find(|g| g.group_name == "Frontend");
        assert_eq!(frontend.unwrap().tabs.len(), 0);
    }

    #[test]
    fn test_unmatched_tab_goes_to_default() {
        let tabs = vec![make_tab("random-tab", 0)];
        let groups = vec![make_group("Frontend", r"^FE\|")];
        let result = assign_groups(
            &tabs,
            &groups,
            &HashMap::new(),
            &HashSet::new(),
            false,
            "index",
        );
        let default_group = result.iter().find(|g| g.group_name == "Default");
        assert!(default_group.is_some());
        assert_eq!(default_group.unwrap().tabs.len(), 1);
    }

    #[test]
    fn test_default_group_ordered_first() {
        let tabs = vec![
            make_tab("FE|x", 0),
            make_tab("BE|y", 1),
            make_tab("other", 2),
        ];
        let groups = vec![
            make_group("Frontend", r"^FE\|"),
            make_group("Backend", r"^BE\|"),
        ];
        let result = assign_groups(
            &tabs,
            &groups,
            &HashMap::new(),
            &HashSet::new(),
            false,
            "index",
        );
        assert_eq!(result[0].group_name, "Default", "Default should come first");
    }

    #[test]
    fn test_configured_groups_sorted_alphabetically() {
        let tabs = vec![make_tab("FE|x", 0), make_tab("BE|y", 1)];
        let groups = vec![
            make_group("Frontend", r"^FE\|"),
            make_group("Backend", r"^BE\|"),
        ];
        let result = assign_groups(
            &tabs,
            &groups,
            &HashMap::new(),
            &HashSet::new(),
            false,
            "index",
        );
        let non_default: Vec<&str> = result
            .iter()
            .map(|g| g.group_name.as_str())
            .filter(|n| *n != "Default")
            .collect();
        assert_eq!(non_default, vec!["Backend", "Frontend"]);
    }

    #[test]
    fn test_collapsed_group_flagged() {
        let tabs = vec![make_tab("FE|x", 0)];
        let groups = vec![make_group("Frontend", r"^FE\|")];
        let mut collapsed = HashSet::new();
        collapsed.insert("Frontend".to_string());
        let result = assign_groups(&tabs, &groups, &HashMap::new(), &collapsed, false, "index");
        let frontend = result.iter().find(|g| g.group_name == "Frontend").unwrap();
        assert!(frontend.collapsed);
    }

    #[test]
    fn test_empty_groups_hidden_by_default() {
        let tabs = vec![make_tab("FE|x", 0)];
        let groups = vec![
            make_group("Frontend", r"^FE\|"),
            make_group("Backend", r"^BE\|"),
        ];
        let result = assign_groups(
            &tabs,
            &groups,
            &HashMap::new(),
            &HashSet::new(),
            false,
            "index",
        );
        assert!(
            result.iter().all(|g| g.group_name != "Backend"),
            "Backend (empty) should be hidden when show_empty=false"
        );
    }

    #[test]
    fn test_empty_groups_shown_when_configured() {
        let tabs = vec![];
        let groups = vec![make_group("Frontend", r"^FE\|")];
        let result = assign_groups(
            &tabs,
            &groups,
            &HashMap::new(),
            &HashSet::new(),
            true,
            "index",
        );
        assert!(
            result.iter().any(|g| g.group_name == "Frontend"),
            "Frontend should appear when show_empty=true"
        );
    }
}
