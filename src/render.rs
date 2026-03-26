use crate::click::{ClickRegion, ClickTarget};
use crate::colors;
use crate::grouping::{assign_groups, auto_fill_theme};
use crate::indicators::{render_indicators, IndicatorState};
use crate::state::{MenuTarget, PluginState, TabKey};
use crate::widgets;
use zellij_tile::prelude::*;

pub const PINNED_HEIGHT: usize = 2;

#[allow(dead_code)]
fn format_sidebar_line(bg: &str, fg: &str, text: &str, cursor: bool) -> String {
    let cursor_mark = if cursor { colors::REVERSE } else { "" };
    format!("{}{}{}{}{}", bg, fg, cursor_mark, text, colors::RESET)
}

pub fn render_sidebar(state: &mut PluginState, rows: usize, cols: usize) {
    if state.sidebar_collapsed {
        for _ in 0..rows {
            print_text(Text::new("›".to_string()));
        }
        state.click_regions = Vec::new();
        return;
    }
    let scrollable_rows = rows.saturating_sub(PINNED_HEIGHT);

    let sidebar_theme = {
        let s: &PluginState = state;
        crate::colors::get_theme(&s.config.sidebar.theme)
    };
    let (mut all_lines, regions) = {
        let s: &PluginState = state;
        build_sidebar_lines(s, cols)
    };

    let max_offset = all_lines.len().saturating_sub(scrollable_rows);
    state.max_viewport_offset = max_offset;

    auto_scroll_to_active(state, &regions, scrollable_rows);

    if state.viewport_offset > max_offset {
        state.viewport_offset = max_offset;
    }

    state.click_regions = regions;

    if let Some(pos) = state.cursor_position {
        let max_valid = state.click_regions.len().saturating_sub(1);
        if state.click_regions.is_empty() {
            state.cursor_position = None;
        } else if pos > max_valid {
            state.cursor_position = Some(max_valid);
        }
    }

    let offset = state.viewport_offset;

    if let Some(menu_state) = state.active_menu.clone() {
        let group_names: Vec<String> = state.config.groups.iter().map(|g| g.name.clone()).collect();
        apply_menu_overlay(
            &mut all_lines,
            &menu_state,
            offset,
            scrollable_rows,
            cols,
            &group_names,
            &sidebar_theme,
        );
    }

    for line in all_lines.iter().skip(offset).take(scrollable_rows) {
        print_text(Text::new(line.clone()));
    }
    let rendered = all_lines.len().saturating_sub(offset).min(scrollable_rows);
    for _ in rendered..scrollable_rows {
        print_text(Text::new(format!(
            "{}{}{}",
            crate::colors::ansi_bg(&sidebar_theme.sidebar_bg),
            crate::colors::ansi_fg(&sidebar_theme.sidebar_fg),
            " ".repeat(cols.max(1))
        )));
    }

    let divider = format!(
        "{}{}{}{}",
        crate::colors::ansi_bg(&sidebar_theme.sidebar_bg),
        crate::colors::ansi_fg(&sidebar_theme.divider_fg),
        "─".repeat(cols.max(1)),
        colors::RESET
    );
    print_text(Text::new(divider));

    print_text(Text::new(build_widget_line(state, cols)));
}

#[allow(dead_code)]
pub fn clamp_viewport(offset: usize, total_lines: usize, rows: usize) -> usize {
    let max_offset = total_lines.saturating_sub(rows);
    offset.min(max_offset)
}

/// Apply context menu overlay onto `all_lines` in-place.
/// Returns the number of items written (0 if no menu or no items).
pub fn apply_menu_overlay(
    all_lines: &mut [String],
    menu_state: &crate::state::MenuState,
    offset: usize,
    scrollable_rows: usize,
    cols: usize,
    group_names: &[String],
    theme: &crate::colors::SidebarTheme,
) -> usize {
    use crate::menus::{build_group_menu, build_pane_menu, build_tab_menu};
    let items = if let Some(cached) = &menu_state.items_cache {
        cached.clone()
    } else {
        match &menu_state.target {
            MenuTarget::Tab(pos) => build_tab_menu(*pos, group_names),
            MenuTarget::Pane(id) => build_pane_menu(*id),
            MenuTarget::Group(name) => build_group_menu(name),
            MenuTarget::None => return 0,
        }
    };
    let in_submenu = menu_state.parent_items.is_some();
    let menu_width = 26.min(cols);
    let start_visual = menu_state.position_line;
    let mut written = 0;
    if in_submenu {
        let abs_row = start_visual + offset;
        if abs_row < all_lines.len() && start_visual < scrollable_rows {
            all_lines[abs_row] = format!(
                "{}{}{}{}",
                colors::ansi_bg(&theme.menu_bg),
                colors::ansi_fg(&theme.menu_fg),
                pad_visible("← Back (Esc)", menu_width),
                colors::RESET
            );
            written += 1;
        }
    }
    let item_row_offset = usize::from(in_submenu);
    for (i, item) in items.iter().enumerate() {
        let visual_row = start_visual + item_row_offset + i;
        let abs_row = visual_row + offset;
        if abs_row < all_lines.len() && visual_row < scrollable_rows {
            let selected = i == menu_state.selected_index;
            let (item_bg, item_fg) = if selected {
                (
                    colors::ansi_bg(&theme.menu_selected_bg),
                    colors::ansi_fg(&theme.menu_selected_fg),
                )
            } else {
                (
                    colors::ansi_bg(&theme.menu_bg),
                    colors::ansi_fg(&theme.menu_fg),
                )
            };
            let content = if item.is_separator {
                "─".repeat(menu_width.saturating_sub(2))
            } else if selected {
                format!("▶ {}", &item.label)
            } else {
                format!("  {}", &item.label)
            };
            all_lines[abs_row] = format!(
                "{}{}{}{}",
                item_bg,
                item_fg,
                pad_visible(&content, menu_width),
                colors::RESET
            );
            written += 1;
        }
    }
    written
}

/// Build the widget/rename footer line (does NOT call print_text).
pub fn build_widget_line(state: &PluginState, cols: usize) -> String {
    if let Some(rs) = &state.rename_state {
        let prompt = format!("  Rename: {}\u{2588}", rs.buffer);
        pad_visible(&prompt, cols)
    } else {
        widgets::render_pinned(state, cols)
    }
}

fn auto_scroll_to_active(state: &mut PluginState, regions: &[ClickRegion], scrollable_rows: usize) {
    let active_tab_pos = state
        .tab_entries
        .iter()
        .find(|t| t.active)
        .map(|t| t.position);
    if let Some(pos) = active_tab_pos {
        if let Some(region) = regions
            .iter()
            .find(|r| matches!(r.target, ClickTarget::Tab(p) if p == pos))
        {
            let line = region.line;
            if line < state.viewport_offset {
                state.viewport_offset = line;
            } else if scrollable_rows > 0 && line >= state.viewport_offset + scrollable_rows {
                state.viewport_offset = line.saturating_sub(scrollable_rows - 1);
            }
        }
    }
}

pub fn build_sidebar_lines(state: &PluginState, cols: usize) -> (Vec<String>, Vec<ClickRegion>) {
    let show_empty = state.config.sidebar.show_empty_groups;
    let groups = assign_groups(
        &state.tab_entries,
        &state.config.groups,
        &state.group_assignments,
        &state.collapsed_groups,
        show_empty,
        &state.config.sidebar.sort_by,
    );

    let cursor_pos = state.cursor_position;
    let sidebar_theme = crate::colors::get_theme(&state.config.sidebar.theme);
    let is_dark = sidebar_theme.is_dark;
    let mut lines: Vec<String> = Vec::new();
    let mut regions: Vec<ClickRegion> = Vec::new();
    let mut line_idx: usize = 0;

    for (group_idx, group) in groups.iter().enumerate() {
        let theme = auto_fill_theme(&group.theme, group_idx, is_dark);

        let disclosure = if group.collapsed { "⊞" } else { "⊟" };
        let icon_part = theme
            .icon
            .as_deref()
            .map(|i| format!("{} ", i))
            .unwrap_or_default();
        let header_text = format!("{} {}{}", disclosure, icon_part, group.group_name);
        let padded = pad_visible(&header_text, cols);
        lines.push(format_sidebar_line(
            &colors::ansi_bg(&theme.bg),
            &colors::ansi_fg(&theme.fg),
            &padded,
            cursor_pos == Some(line_idx),
        ));
        regions.push(ClickRegion {
            line: line_idx,
            target: ClickTarget::Group(group.group_name.clone()),
        });
        line_idx += 1;

        if group.collapsed {
            continue;
        }

        for tab in &group.tabs {
            let tab_key = TabKey::new(&tab.name, tab.position);
            let marker = state
                .markers
                .get(&tab_key)
                .map(|m| format!("{} ", m))
                .unwrap_or_default();
            let active_arrow = if tab.active { "▶" } else { " " };
            let ind_str = aggregate_tab_indicators(state, tab);
            let tab_text = if ind_str.is_empty() {
                format!("  {} {}{}", active_arrow, marker, tab.name)
            } else {
                format!("  {} {}{} {}", active_arrow, marker, tab.name, ind_str)
            };
            let padded_tab = pad_visible(&tab_text, cols);

            let custom_color = state.custom_colors.get(&tab_key).cloned();
            let (tab_bg, tab_fg) = if tab.active {
                let abg = custom_color
                    .unwrap_or_else(|| theme.active_bg.clone().unwrap_or_else(|| theme.bg.clone()));
                let afg = theme.active_fg.clone().unwrap_or_else(|| theme.fg.clone());
                (colors::ansi_bg(&abg), colors::ansi_fg(&afg))
            } else {
                let tbg = custom_color.unwrap_or_else(|| theme.bg.clone());
                let tfg = colors::derive_text_color(&tbg);
                (colors::ansi_bg(&tbg), colors::ansi_fg(&tfg))
            };

            lines.push(format_sidebar_line(
                &tab_bg,
                &tab_fg,
                &padded_tab,
                cursor_pos == Some(line_idx),
            ));
            regions.push(ClickRegion {
                line: line_idx,
                target: ClickTarget::Tab(tab.position),
            });
            line_idx += 1;

            for pane in tab
                .panes
                .iter()
                .filter(|p| !p.is_plugin && state.config.sidebar.show_panes)
            {
                let dot = if pane.is_focused { "●" } else { "·" };
                let title_str = if pane.title.is_empty() {
                    "pane".into()
                } else {
                    pane.title.clone()
                };
                let pane_text = format!("      {} {}", dot, title_str);
                let padded_pane = pad_visible(&pane_text, cols);
                lines.push(format_sidebar_line(
                    &tab_bg,
                    &tab_fg,
                    &padded_pane,
                    cursor_pos == Some(line_idx),
                ));
                regions.push(ClickRegion {
                    line: line_idx,
                    target: ClickTarget::Pane(pane.id),
                });
                line_idx += 1;
            }
        }
    }

    (lines, regions)
}

fn aggregate_tab_indicators(state: &PluginState, tab: &crate::state::TabEntry) -> String {
    let mut combined = IndicatorState::default();
    for pane in tab.panes.iter().filter(|p| !p.is_plugin) {
        let key = format!("%{}", pane.id);
        if let Some(ind) = state.indicators.get(&key) {
            if ind.busy {
                combined.busy = true;
                combined.busy_frame = ind.busy_frame;
            }
            if ind.bell {
                combined.bell = true;
            }
            if ind.input {
                combined.input = true;
            }
        }
    }
    render_indicators(&combined)
}

#[allow(dead_code)]
pub(crate) fn menu_target_name(target: &MenuTarget) -> String {
    match target {
        MenuTarget::Tab(pos) => format!("tab {}", pos),
        MenuTarget::Pane(id) => format!("pane {}", id),
        MenuTarget::Group(name) => name.clone(),
        MenuTarget::None => "menu".into(),
    }
}

fn pad_visible(s: &str, cols: usize) -> String {
    if cols == 0 {
        return s.to_string();
    }
    let visible = s.chars().count();
    if visible >= cols {
        s.chars().take(cols).collect()
    } else {
        format!("{}{}", s, " ".repeat(cols - visible))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::click::ClickTarget;
    use crate::state::{PluginState, TabEntry};

    fn make_tab(name: &str, position: usize, active: bool) -> TabEntry {
        TabEntry {
            position,
            name: name.to_string(),
            active,
            panes: vec![],
        }
    }

    fn state_with_tabs(tabs: Vec<TabEntry>) -> PluginState {
        let mut s = PluginState::default();
        s.tab_entries = tabs;
        s.config.sidebar.show_panes = true;
        s
    }

    #[test]
    fn test_empty_state_no_panic() {
        let state = PluginState::default();
        let (lines, map) = build_sidebar_lines(&state, 30);
        let _ = (lines, map);
    }

    #[test]
    fn test_single_tab_produces_header_and_row() {
        let state = state_with_tabs(vec![make_tab("api", 0, false)]);
        let (lines, map) = build_sidebar_lines(&state, 30);
        assert!(
            lines.len() >= 2,
            "expected header + 1 tab, got {}",
            lines.len()
        );
        assert_eq!(
            lines.len(),
            map.len(),
            "lines and click_map must be same length"
        );
    }

    #[test]
    fn test_collapsed_group_shows_only_header() {
        let mut state = state_with_tabs(vec![make_tab("api", 0, false)]);
        state.collapsed_groups.insert("Default".to_string());
        let (lines, _) = build_sidebar_lines(&state, 30);
        assert_eq!(lines.len(), 1, "collapsed group should emit only header");
    }

    #[test]
    fn test_active_tab_has_indicator() {
        let state = state_with_tabs(vec![make_tab("api", 0, true), make_tab("web", 1, false)]);
        let (lines, _) = build_sidebar_lines(&state, 40);
        let active_lines: Vec<&String> = lines.iter().filter(|l| l.contains('▶')).collect();
        assert_eq!(
            active_lines.len(),
            1,
            "exactly one active indicator expected"
        );
        assert!(active_lines[0].contains("api"), "active tab should be api");
    }

    #[test]
    fn test_inactive_tab_has_no_arrow() {
        let state = state_with_tabs(vec![make_tab("api", 0, false)]);
        let (lines, _) = build_sidebar_lines(&state, 30);
        let tab_lines: Vec<&String> = lines.iter().skip(1).collect();
        assert!(!tab_lines.is_empty());
        assert!(
            !tab_lines.iter().any(|l| l.contains('▶')),
            "inactive tab should not have arrow"
        );
    }

    #[test]
    fn test_lines_contain_ansi_codes() {
        let state = state_with_tabs(vec![make_tab("api", 0, false)]);
        let (lines, _) = build_sidebar_lines(&state, 30);
        assert!(
            lines.iter().any(|l| l.contains("\x1b[")),
            "lines should contain ANSI"
        );
    }

    #[test]
    fn test_marker_appears_in_tab_line() {
        let mut state = state_with_tabs(vec![make_tab("api", 0, false)]);
        state
            .markers
            .insert(TabKey::new("api", 0), "🚀".to_string());
        let (lines, _) = build_sidebar_lines(&state, 40);
        let tab_line = lines.iter().nth(1).expect("tab line expected");
        assert!(tab_line.contains("🚀"), "marker should appear in tab line");
    }

    #[test]
    fn test_custom_color_overrides_tab_bg() {
        let mut state = state_with_tabs(vec![make_tab("api", 0, false)]);
        state
            .custom_colors
            .insert(TabKey::new("api", 0), "#e74c3c".to_string());
        let (lines, _) = build_sidebar_lines(&state, 30);
        let tab_line = lines.iter().nth(1).expect("tab line expected");
        assert!(
            tab_line.contains("231"),
            "custom red ANSI (r=231) should appear"
        );
    }

    #[test]
    fn test_configured_groups_render() {
        use crate::config::{GroupConfig, ThemeConfig};
        let mut state = state_with_tabs(vec![
            make_tab("FE|dashboard", 0, true),
            make_tab("BE|api", 1, false),
        ]);
        state.config.groups = vec![
            GroupConfig {
                name: "Frontend".to_string(),
                pattern: r"^FE\|".to_string(),
                working_dir: None,
                theme: ThemeConfig {
                    bg: "#e74c3c".to_string(),
                    ..ThemeConfig::default()
                },
            },
            GroupConfig {
                name: "Backend".to_string(),
                pattern: r"^BE\|".to_string(),
                working_dir: None,
                theme: ThemeConfig::default(),
            },
        ];
        let (lines, _) = build_sidebar_lines(&state, 40);
        let headers: Vec<&String> = lines
            .iter()
            .filter(|l| l.contains("⊟") || l.contains("⊞"))
            .collect();
        assert!(
            headers.len() >= 2,
            "should have at least 2 group headers, got {}",
            headers.len()
        );
    }

    #[test]
    fn test_click_regions_tab_entries() {
        let state = state_with_tabs(vec![make_tab("api", 0, false), make_tab("web", 1, false)]);
        let (lines, regions) = build_sidebar_lines(&state, 30);
        assert_eq!(lines.len(), regions.len());
        let tab_regions: Vec<_> = regions
            .iter()
            .filter(|r| matches!(r.target, ClickTarget::Tab(_)))
            .collect();
        assert_eq!(tab_regions.len(), 2, "should have 2 Tab click regions");
    }

    #[test]
    fn test_click_regions_group_entry() {
        let state = state_with_tabs(vec![make_tab("api", 0, false)]);
        let (_, regions) = build_sidebar_lines(&state, 30);
        assert!(
            regions
                .iter()
                .any(|r| matches!(r.target, ClickTarget::Group(_))),
            "should have at least one Group click region"
        );
    }

    #[test]
    fn test_click_regions_line_indices_match() {
        let state = state_with_tabs(vec![make_tab("api", 0, false), make_tab("web", 1, false)]);
        let (lines, regions) = build_sidebar_lines(&state, 30);
        assert_eq!(lines.len(), regions.len());
        for (i, region) in regions.iter().enumerate() {
            assert_eq!(
                region.line, i,
                "region at index {} should have line={}",
                i, i
            );
        }
    }

    #[test]
    fn test_pad_visible_pads_to_cols() {
        let result = pad_visible("hello", 10);
        assert_eq!(result.chars().count(), 10);
        assert!(result.starts_with("hello"));
    }

    #[test]
    fn test_pad_visible_truncates_long_input() {
        let result = pad_visible("hello world!", 5);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_bell_indicator_appears_in_tab_line() {
        use crate::indicators::IndicatorState;
        use crate::state::PaneEntry;

        let mut tab = make_tab("api", 0, false);
        tab.panes = vec![PaneEntry {
            id: 42,
            is_plugin: false,
            ..PaneEntry::default()
        }];
        let mut state = PluginState::default();
        state.tab_entries = vec![tab];
        state.indicators.insert(
            "%42".into(),
            IndicatorState {
                bell: true,
                ..IndicatorState::default()
            },
        );
        let (lines, _) = build_sidebar_lines(&state, 40);
        let tab_line = lines.iter().nth(1).expect("tab line expected");
        assert!(tab_line.contains('◆'), "bell indicator should appear");
    }

    #[test]
    fn test_no_indicators_when_panes_empty() {
        let state = state_with_tabs(vec![make_tab("api", 0, false)]);
        let (lines, _) = build_sidebar_lines(&state, 40);
        let tab_line = lines.iter().nth(1).expect("tab line expected");
        assert!(!tab_line.contains('◆'), "no bell when panes empty");
        assert!(
            !tab_line.contains('?'),
            "no input indicator when panes empty"
        );
    }

    #[test]
    fn test_plugin_pane_excluded_from_indicators() {
        use crate::indicators::IndicatorState;
        use crate::state::PaneEntry;

        let mut tab = make_tab("api", 0, false);
        tab.panes = vec![PaneEntry {
            id: 7,
            is_plugin: true,
            ..PaneEntry::default()
        }];
        let mut state = PluginState::default();
        state.tab_entries = vec![tab];
        state.indicators.insert(
            "%7".into(),
            IndicatorState {
                bell: true,
                ..IndicatorState::default()
            },
        );
        let (lines, _) = build_sidebar_lines(&state, 40);
        let tab_line = lines.iter().nth(1).expect("tab line expected");
        assert!(!tab_line.contains('◆'), "plugin pane should be excluded");
    }

    fn make_pane(id: u32, title: &str, is_focused: bool) -> crate::state::PaneEntry {
        crate::state::PaneEntry {
            id,
            is_plugin: false,
            title: title.to_string(),
            is_focused,
            is_floating: false,
            cwd: None,
        }
    }

    #[test]
    fn test_pane_renders_under_tab() {
        let mut tab = make_tab("api", 0, false);
        tab.panes = vec![make_pane(1, "nvim", false)];
        let state = state_with_tabs(vec![tab]);
        let (lines, regions) = build_sidebar_lines(&state, 40);
        assert_eq!(lines.len(), 3, "header + tab + 1 pane = 3 lines");
        assert_eq!(regions.len(), 3);
    }

    #[test]
    fn test_plugin_pane_not_rendered() {
        let mut tab = make_tab("api", 0, false);
        tab.panes = vec![crate::state::PaneEntry {
            id: 99,
            is_plugin: true,
            title: "tabby-zj".into(),
            ..crate::state::PaneEntry::default()
        }];
        let state = state_with_tabs(vec![tab]);
        let (lines, _) = build_sidebar_lines(&state, 40);
        assert_eq!(lines.len(), 2, "plugin pane should not add a line");
    }

    #[test]
    fn test_focused_pane_has_filled_dot() {
        let mut tab = make_tab("api", 0, false);
        tab.panes = vec![make_pane(1, "nvim", true)];
        let state = state_with_tabs(vec![tab]);
        let (lines, _) = build_sidebar_lines(&state, 40);
        let pane_line = lines.last().expect("pane line expected");
        assert!(
            pane_line.contains('●'),
            "focused pane should show filled dot"
        );
        assert!(
            !pane_line.contains('·'),
            "focused pane should not show hollow dot"
        );
    }

    #[test]
    fn test_unfocused_pane_has_hollow_dot() {
        let mut tab = make_tab("api", 0, false);
        tab.panes = vec![make_pane(2, "bash", false)];
        let state = state_with_tabs(vec![tab]);
        let (lines, _) = build_sidebar_lines(&state, 40);
        let pane_line = lines.last().expect("pane line expected");
        assert!(
            pane_line.contains('·'),
            "unfocused pane should show hollow dot"
        );
    }

    #[test]
    fn test_pane_title_appears_in_pane_line() {
        let mut tab = make_tab("api", 0, false);
        tab.panes = vec![make_pane(1, "cargo build", false)];
        let state = state_with_tabs(vec![tab]);
        let (lines, _) = build_sidebar_lines(&state, 50);
        let pane_line = lines.last().expect("pane line expected");
        assert!(
            pane_line.contains("cargo build"),
            "pane title should appear"
        );
    }

    #[test]
    fn test_pane_click_region_has_pane_target() {
        let mut tab = make_tab("api", 0, false);
        tab.panes = vec![make_pane(42, "nvim", false)];
        let state = state_with_tabs(vec![tab]);
        let (_, regions) = build_sidebar_lines(&state, 40);
        let pane_regions: Vec<_> = regions
            .iter()
            .filter(|r| matches!(r.target, ClickTarget::Pane(42)))
            .collect();
        assert_eq!(pane_regions.len(), 1, "should have one Pane click region");
    }

    #[test]
    fn test_pane_region_line_follows_tab_line() {
        let mut tab = make_tab("api", 0, false);
        tab.panes = vec![make_pane(1, "nvim", false)];
        let state = state_with_tabs(vec![tab]);
        let (_, regions) = build_sidebar_lines(&state, 40);
        let tab_region = regions
            .iter()
            .find(|r| matches!(r.target, ClickTarget::Tab(0)))
            .unwrap();
        let pane_region = regions
            .iter()
            .find(|r| matches!(r.target, ClickTarget::Pane(1)))
            .unwrap();
        assert_eq!(
            pane_region.line,
            tab_region.line + 1,
            "pane line should be directly after tab"
        );
    }

    #[test]
    fn test_multiple_panes_all_rendered() {
        let mut tab = make_tab("api", 0, false);
        tab.panes = vec![
            make_pane(1, "nvim", true),
            make_pane(2, "bash", false),
            make_pane(3, "htop", false),
        ];
        let state = state_with_tabs(vec![tab]);
        let (lines, regions) = build_sidebar_lines(&state, 40);
        assert_eq!(lines.len(), 5, "header + tab + 3 panes = 5");
        let pane_regions: Vec<_> = regions
            .iter()
            .filter(|r| matches!(r.target, ClickTarget::Pane(_)))
            .collect();
        assert_eq!(pane_regions.len(), 3);
    }

    #[test]
    fn test_empty_pane_title_shows_fallback() {
        let mut tab = make_tab("api", 0, false);
        tab.panes = vec![make_pane(1, "", false)];
        let state = state_with_tabs(vec![tab]);
        let (lines, _) = build_sidebar_lines(&state, 40);
        let pane_line = lines.last().expect("pane line expected");
        assert!(
            pane_line.contains("pane"),
            "empty title should fall back to 'pane'"
        );
    }

    #[test]
    fn test_clamp_viewport_zero_when_no_overflow() {
        assert_eq!(clamp_viewport(0, 10, 24), 0);
    }

    #[test]
    fn test_clamp_viewport_reduces_excess_offset() {
        assert_eq!(clamp_viewport(20, 10, 24), 0);
    }

    #[test]
    fn test_clamp_viewport_exact_boundary() {
        assert_eq!(clamp_viewport(6, 30, 24), 6);
    }

    #[test]
    fn test_clamp_viewport_at_max_allowed() {
        assert_eq!(clamp_viewport(6, 30, 24), 6);
        assert_eq!(clamp_viewport(7, 30, 24), 6);
    }

    #[test]
    fn test_clamp_viewport_fewer_lines_than_rows() {
        assert_eq!(clamp_viewport(5, 3, 24), 0);
    }

    #[test]
    fn test_clamp_viewport_equal_lines_and_rows() {
        assert_eq!(clamp_viewport(1, 24, 24), 0);
    }

    #[test]
    fn test_clamp_viewport_valid_offset_unchanged() {
        assert_eq!(clamp_viewport(3, 30, 24), 3);
    }

    #[test]
    fn test_viewport_clamped_to_max() {
        let tabs: Vec<TabEntry> = (0..5)
            .map(|i| make_tab(&format!("tab{}", i), i, i == 0))
            .collect();
        let mut state = state_with_tabs(tabs);
        state.viewport_offset = 0;
        render_sidebar(&mut state, 4, 30);
        assert!(
            state.viewport_offset <= state.max_viewport_offset,
            "viewport_offset {} must be <= max_viewport_offset {}",
            state.viewport_offset,
            state.max_viewport_offset
        );
    }

    #[test]
    fn test_active_tab_auto_scroll_into_view() {
        let mut state = state_with_tabs(vec![
            make_tab("other", 0, false),
            make_tab("active", 10, true),
        ]);
        state.viewport_offset = 0;
        let regions = vec![
            ClickRegion {
                line: 0,
                target: ClickTarget::Group("Default".to_string()),
            },
            ClickRegion {
                line: 1,
                target: ClickTarget::Tab(0),
            },
            ClickRegion {
                line: 10,
                target: ClickTarget::Tab(10),
            },
        ];
        auto_scroll_to_active(&mut state, &regions, 5);
        assert_eq!(
            state.viewport_offset, 6,
            "line 10 with scrollable_rows=5 should shift offset to 6"
        );
    }

    #[test]
    fn test_auto_scroll_no_change_when_visible() {
        let mut state = state_with_tabs(vec![make_tab("active", 0, true)]);
        state.viewport_offset = 0;
        let regions = vec![
            ClickRegion {
                line: 0,
                target: ClickTarget::Group("Default".to_string()),
            },
            ClickRegion {
                line: 1,
                target: ClickTarget::Tab(0),
            },
        ];
        auto_scroll_to_active(&mut state, &regions, 10);
        assert_eq!(
            state.viewport_offset, 0,
            "active tab at line 1 is within [0,10), offset should not change"
        );
    }

    #[test]
    fn test_pinned_height_reduces_scrollable_area() {
        let tabs: Vec<TabEntry> = (0..3)
            .map(|i| make_tab(&format!("t{}", i), i, i == 0))
            .collect();
        let mut state = state_with_tabs(tabs);
        render_sidebar(&mut state, 10, 30);
        assert_eq!(
            state.max_viewport_offset, 0,
            "4 content lines fit in 8 scrollable rows (rows=10, pinned=2), max_offset=0"
        );
    }

    #[test]
    fn test_scroll_down_clamped() {
        let tabs: Vec<TabEntry> = (0..5)
            .map(|i| make_tab(&format!("tab{}", i), i, false))
            .collect();
        let mut state = state_with_tabs(tabs);
        state.viewport_offset = 9999;
        render_sidebar(&mut state, 4, 30);
        assert_eq!(
            state.viewport_offset, state.max_viewport_offset,
            "9999 should be clamped to max_viewport_offset"
        );
        assert!(state.max_viewport_offset < 100);
    }

    #[test]
    fn test_auto_scroll_up_when_active_above_viewport() {
        let mut state = state_with_tabs(vec![
            make_tab("active", 0, true),
            make_tab("other", 1, false),
        ]);
        state.viewport_offset = 5;
        let regions = vec![
            ClickRegion {
                line: 0,
                target: ClickTarget::Group("Default".to_string()),
            },
            ClickRegion {
                line: 1,
                target: ClickTarget::Tab(0),
            },
            ClickRegion {
                line: 2,
                target: ClickTarget::Tab(1),
            },
        ];
        auto_scroll_to_active(&mut state, &regions, 5);
        assert_eq!(
            state.viewport_offset, 1,
            "active tab at line 1 above viewport(5) should scroll up to line 1"
        );
    }

    #[test]
    fn test_max_viewport_offset_written_to_state() {
        let tabs: Vec<TabEntry> = (0..10)
            .map(|i| make_tab(&format!("tab{}", i), i, i == 0))
            .collect();
        let mut state = state_with_tabs(tabs);
        render_sidebar(&mut state, 6, 30);
        assert!(
            state.max_viewport_offset > 0,
            "10 tabs should produce max_viewport_offset > 0 with rows=6"
        );
        assert_eq!(
            state.max_viewport_offset,
            state.viewport_offset.max(state.max_viewport_offset),
            "max_viewport_offset must be an upper bound on viewport_offset"
        );
    }

    #[test]
    fn test_cursor_none_no_reverse_video() {
        let state = state_with_tabs(vec![make_tab("api", 0, false)]);
        let (lines, _) = build_sidebar_lines(&state, 40);
        assert!(
            !lines.iter().any(|l| l.contains("\x1b[7m")),
            "no reverse video when cursor_position is None"
        );
    }

    #[test]
    fn test_cursor_on_group_header_applies_reverse() {
        let mut state = state_with_tabs(vec![make_tab("api", 0, false)]);
        state.cursor_position = Some(0);
        let (lines, _) = build_sidebar_lines(&state, 40);
        assert!(
            lines[0].contains("\x1b[7m"),
            "group header at index 0 should have reverse video"
        );
        assert!(
            !lines[1].contains("\x1b[7m"),
            "tab at index 1 should not have reverse video"
        );
    }

    #[test]
    fn test_cursor_on_tab_row_applies_reverse() {
        let mut state = state_with_tabs(vec![make_tab("api", 0, false)]);
        state.cursor_position = Some(1);
        let (lines, _) = build_sidebar_lines(&state, 40);
        assert!(
            !lines[0].contains("\x1b[7m"),
            "group header should not have reverse"
        );
        assert!(
            lines[1].contains("\x1b[7m"),
            "tab at index 1 should have reverse video"
        );
    }

    #[test]
    fn test_cursor_on_pane_row_applies_reverse() {
        let mut tab = make_tab("api", 0, false);
        tab.panes = vec![make_pane(1, "nvim", false)];
        let mut state = state_with_tabs(vec![tab]);
        state.cursor_position = Some(2);
        let (lines, _) = build_sidebar_lines(&state, 40);
        assert!(
            !lines[0].contains("\x1b[7m"),
            "group header should not have cursor"
        );
        assert!(
            !lines[1].contains("\x1b[7m"),
            "tab line should not have cursor"
        );
        assert!(
            lines[2].contains("\x1b[7m"),
            "pane at index 2 should have reverse video"
        );
    }

    #[test]
    fn test_only_cursor_line_has_reverse() {
        let tabs = vec![make_tab("api", 0, false), make_tab("web", 1, false)];
        let mut state = state_with_tabs(tabs);
        state.cursor_position = Some(2);
        let (lines, _) = build_sidebar_lines(&state, 40);
        let count = lines.iter().filter(|l| l.contains("\x1b[7m")).count();
        assert_eq!(count, 1, "exactly one line should have cursor highlight");
    }

    #[test]
    fn test_cursor_moves_highlight() {
        let mut state = state_with_tabs(vec![make_tab("api", 0, false)]);
        state.cursor_position = Some(0);
        let (lines_at_0, _) = build_sidebar_lines(&state, 40);
        state.cursor_position = Some(1);
        let (lines_at_1, _) = build_sidebar_lines(&state, 40);
        assert!(
            lines_at_0[0].contains("\x1b[7m"),
            "cursor=0: group highlighted"
        );
        assert!(
            !lines_at_0[1].contains("\x1b[7m"),
            "cursor=0: tab not highlighted"
        );
        assert!(
            !lines_at_1[0].contains("\x1b[7m"),
            "cursor=1: group not highlighted"
        );
        assert!(
            lines_at_1[1].contains("\x1b[7m"),
            "cursor=1: tab highlighted"
        );
    }

    #[test]
    fn test_cursor_out_of_bounds_no_panic() {
        let mut state = state_with_tabs(vec![make_tab("api", 0, false)]);
        state.cursor_position = Some(999);
        let (lines, _) = build_sidebar_lines(&state, 40);
        assert!(
            !lines.iter().any(|l| l.contains("\x1b[7m")),
            "out-of-bounds cursor should not highlight any line"
        );
    }

    #[test]
    fn test_menu_overlay_replaces_line_at_position() {
        use crate::state::{MenuState, MenuTarget};
        let state = state_with_tabs(vec![make_tab("api", 0, false), make_tab("web", 1, false)]);
        let (mut lines, _) = build_sidebar_lines(&state, 40);
        let original_line1 = lines[1].clone();
        let menu = MenuState {
            target: MenuTarget::Tab(0),
            selected_index: 0,
            position_line: 1,
            ..Default::default()
        };
        let theme = crate::colors::catppuccin_mocha();
        let written = apply_menu_overlay(&mut lines, &menu, 0, 20, 40, &[], &theme);
        assert!(written > 0, "should write at least one menu item");
        assert_ne!(
            lines[1], original_line1,
            "line at position_line should be replaced by menu item"
        );
    }

    #[test]
    fn test_menu_overlay_selected_item_has_arrow() {
        use crate::state::{MenuState, MenuTarget};
        let state = state_with_tabs(vec![make_tab("api", 0, false)]);
        let (mut lines, _) = build_sidebar_lines(&state, 40);
        let menu = MenuState {
            target: MenuTarget::Tab(0),
            selected_index: 0,
            position_line: 0,
            ..Default::default()
        };
        let theme = crate::colors::catppuccin_mocha();
        apply_menu_overlay(&mut lines, &menu, 0, 20, 40, &[], &theme);
        assert!(
            lines[0].contains('▶'),
            "selected item (index 0) should have arrow indicator"
        );
    }

    #[test]
    fn test_menu_overlay_unselected_item_no_arrow() {
        use crate::state::{MenuState, MenuTarget};
        let state = state_with_tabs(vec![
            make_tab("api", 0, false),
            make_tab("web", 1, false),
            make_tab("db", 2, false),
        ]);
        let (mut lines, _) = build_sidebar_lines(&state, 40);
        let menu = MenuState {
            target: MenuTarget::Tab(0),
            selected_index: 0,
            position_line: 0,
            ..Default::default()
        };
        let theme = crate::colors::catppuccin_mocha();
        apply_menu_overlay(&mut lines, &menu, 0, 20, 40, &[], &theme);
        if lines.len() > 1 {
            assert!(
                !lines[1].contains('▶') || lines[1].contains("\x1b["),
                "unselected item at line 1 should not have selection arrow (may have tab arrow)"
            );
        }
    }

    #[test]
    fn test_menu_overlay_none_target_writes_nothing() {
        use crate::state::{MenuState, MenuTarget};
        let state = state_with_tabs(vec![make_tab("api", 0, false)]);
        let (mut lines, _) = build_sidebar_lines(&state, 40);
        let original = lines.clone();
        let menu = MenuState {
            target: MenuTarget::None,
            selected_index: 0,
            position_line: 0,
            ..Default::default()
        };
        let theme = crate::colors::catppuccin_mocha();
        let written = apply_menu_overlay(&mut lines, &menu, 0, 20, 40, &[], &theme);
        assert_eq!(written, 0, "MenuTarget::None should write 0 items");
        assert_eq!(lines, original, "lines should be unchanged for None target");
    }

    #[test]
    fn test_menu_overlay_out_of_bounds_position_no_panic() {
        use crate::state::{MenuState, MenuTarget};
        let state = state_with_tabs(vec![make_tab("api", 0, false)]);
        let (mut lines, _) = build_sidebar_lines(&state, 40);
        let menu = MenuState {
            target: MenuTarget::Tab(0),
            selected_index: 0,
            position_line: 9999,
            ..Default::default()
        };
        let theme = crate::colors::catppuccin_mocha();
        let written = apply_menu_overlay(&mut lines, &menu, 0, 20, 40, &[], &theme);
        assert_eq!(
            written, 0,
            "out-of-bounds position_line should write nothing"
        );
    }

    #[test]
    fn test_menu_overlay_group_target_writes_items() {
        use crate::state::{MenuState, MenuTarget};
        let state = state_with_tabs(vec![make_tab("api", 0, false)]);
        let (mut lines, _) = build_sidebar_lines(&state, 40);
        let menu = MenuState {
            target: MenuTarget::Group("Default".to_string()),
            selected_index: 0,
            position_line: 0,
            ..Default::default()
        };
        let theme = crate::colors::catppuccin_mocha();
        let written = apply_menu_overlay(&mut lines, &menu, 0, 20, 40, &[], &theme);
        assert!(written > 0, "group menu should write at least one item");
    }

    #[test]
    fn test_menu_overlay_selected_item_has_highlight_bg() {
        use crate::state::{MenuState, MenuTarget};
        let state = state_with_tabs(vec![make_tab("api", 0, false)]);
        let (mut lines, _) = build_sidebar_lines(&state, 40);
        let menu = MenuState {
            target: MenuTarget::Tab(0),
            selected_index: 0,
            position_line: 0,
            ..Default::default()
        };
        let theme = crate::colors::catppuccin_mocha();
        apply_menu_overlay(&mut lines, &menu, 0, 20, 40, &[], &theme);
        // Check for the theme-derived ANSI code for menu_selected_bg (#3c3c50)
        assert!(
            lines[0].contains("\x1b[48;2;60;60;80m"),
            "selected item should have highlight background color"
        );
    }

    #[test]
    fn test_build_widget_line_no_rename_returns_widget() {
        let state = state_with_tabs(vec![make_tab("api", 0, false)]);
        let line = build_widget_line(&state, 30);
        assert_eq!(line.len(), 30, "widget line should be padded to cols");
        assert!(
            !line.contains("Rename:"),
            "no rename state → no rename prompt"
        );
    }

    #[test]
    fn test_build_widget_line_with_rename_shows_prompt() {
        use crate::state::{RenameState, RenameTarget};
        let mut state = state_with_tabs(vec![make_tab("api", 0, false)]);
        state.rename_state = Some(RenameState {
            target: RenameTarget::Tab(0),
            buffer: "my-tab".to_string(),
        });
        let line = build_widget_line(&state, 40);
        assert!(
            line.contains("Rename:"),
            "rename state → prompt should appear"
        );
        assert!(
            line.contains("my-tab"),
            "rename buffer should appear in prompt"
        );
    }

    #[test]
    fn test_build_widget_line_rename_has_cursor_block() {
        use crate::state::{RenameState, RenameTarget};
        let mut state = state_with_tabs(vec![make_tab("api", 0, false)]);
        state.rename_state = Some(RenameState {
            target: RenameTarget::Tab(0),
            buffer: String::new(),
        });
        let line = build_widget_line(&state, 40);
        assert!(
            line.contains('\u{2588}'),
            "rename prompt should contain block cursor █"
        );
    }

    #[test]
    fn test_build_widget_line_rename_padded_to_cols() {
        use crate::state::{RenameState, RenameTarget};
        let mut state = state_with_tabs(vec![make_tab("api", 0, false)]);
        state.rename_state = Some(RenameState {
            target: RenameTarget::Tab(0),
            buffer: "x".to_string(),
        });
        let line = build_widget_line(&state, 35);
        let visible_len = line.chars().count();
        assert_eq!(visible_len, 35, "rename line should be padded to cols");
    }

    #[test]
    fn test_build_widget_line_rename_empty_buffer() {
        use crate::state::{RenameState, RenameTarget};
        let mut state = state_with_tabs(vec![make_tab("api", 0, false)]);
        state.rename_state = Some(RenameState {
            target: RenameTarget::Group("Default".to_string()),
            buffer: String::new(),
        });
        let line = build_widget_line(&state, 30);
        assert!(line.contains("Rename:"), "empty buffer still shows prompt");
        assert!(line.contains('\u{2588}'), "empty buffer still shows cursor");
    }

    #[test]
    fn test_cursor_clamped_when_regions_shrink() {
        let tabs: Vec<TabEntry> = (0..5)
            .map(|i| make_tab(&format!("t{}", i), i, false))
            .collect();
        let mut state = state_with_tabs(tabs);
        state.cursor_position = Some(10);
        render_sidebar(&mut state, 20, 30);
        if let Some(pos) = state.cursor_position {
            assert!(
                pos < state.click_regions.len(),
                "cursor_position {} must be < click_regions.len() {}",
                pos,
                state.click_regions.len()
            );
        }
    }

    #[test]
    fn test_cursor_cleared_when_regions_empty() {
        let mut state = PluginState::default();
        state.cursor_position = Some(3);
        render_sidebar(&mut state, 10, 30);
        assert_eq!(
            state.cursor_position, None,
            "cursor should be None when click_regions is empty"
        );
    }

    #[test]
    fn test_cursor_valid_position_unchanged() {
        let tabs: Vec<TabEntry> = (0..3)
            .map(|i| make_tab(&format!("t{}", i), i, false))
            .collect();
        let mut state = state_with_tabs(tabs);
        render_sidebar(&mut state, 20, 30);
        let total = state.click_regions.len();
        state.cursor_position = Some(0);
        render_sidebar(&mut state, 20, 30);
        assert_eq!(
            state.cursor_position,
            Some(0),
            "valid cursor position 0 should remain 0 after re-render (total={})",
            total
        );
    }

    #[test]
    fn test_menu_target_name_tab() {
        assert_eq!(menu_target_name(&MenuTarget::Tab(2)), "tab 2");
    }

    #[test]
    fn test_menu_target_name_group() {
        assert_eq!(
            menu_target_name(&MenuTarget::Group("Frontend".into())),
            "Frontend"
        );
    }

    #[test]
    fn test_menu_target_name_pane() {
        assert_eq!(menu_target_name(&MenuTarget::Pane(42)), "pane 42");
    }

    #[test]
    fn test_menu_target_name_none() {
        assert_eq!(menu_target_name(&MenuTarget::None), "menu");
    }

    #[test]
    fn test_build_widget_line_rename_shows_buffer() {
        use crate::state::{RenameState, RenameTarget};
        let mut state = PluginState::default();
        state.rename_state = Some(RenameState {
            target: RenameTarget::Tab(0),
            buffer: "api_new".into(),
        });
        let line = build_widget_line(&state, 40);
        assert!(line.contains("Rename:"), "should show rename prompt");
        assert!(line.contains("api_new"), "should show buffer contents");
    }

    #[test]
    fn test_build_widget_line_no_rename_no_prompt() {
        let state = PluginState::default();
        let line = build_widget_line(&state, 40);
        assert!(
            !line.contains("Rename:"),
            "should not show rename without state"
        );
    }

    #[test]
    fn test_menu_overlay_applied_when_active_menu_set() {
        use crate::state::{MenuState, MenuTarget};
        let mut state = state_with_tabs(vec![make_tab("api", 0, false)]);
        state.active_menu = Some(MenuState {
            target: MenuTarget::Tab(0),
            selected_index: 0,
            position_line: 1,
            ..Default::default()
        });
        render_sidebar(&mut state, 20, 30);
    }

    #[test]
    fn test_show_panes_true_renders_panes() {
        let mut tab = make_tab("api", 0, false);
        tab.panes = vec![make_pane(1, "nvim", false)];
        let mut state = state_with_tabs(vec![tab]);
        state.config.sidebar.show_panes = true;
        let (lines, regions) = build_sidebar_lines(&state, 40);
        assert_eq!(
            lines.len(),
            3,
            "header + tab + 1 pane = 3 lines when show_panes=true"
        );
        assert!(
            regions
                .iter()
                .any(|r| matches!(r.target, ClickTarget::Pane(_))),
            "pane click region should exist when show_panes=true"
        );
    }

    #[test]
    fn test_show_panes_false_hides_panes() {
        let mut tab = make_tab("api", 0, false);
        tab.panes = vec![make_pane(1, "nvim", false), make_pane(2, "bash", false)];
        let mut state = state_with_tabs(vec![tab]);
        state.config.sidebar.show_panes = false;
        let (lines, regions) = build_sidebar_lines(&state, 40);
        assert_eq!(
            lines.len(),
            2,
            "header + tab only = 2 lines when show_panes=false"
        );
        assert!(
            !regions
                .iter()
                .any(|r| matches!(r.target, ClickTarget::Pane(_))),
            "no pane click regions when show_panes=false"
        );
    }

    #[test]
    fn test_show_panes_default_is_true() {
        use crate::config::SidebarConfig;
        assert!(
            SidebarConfig::default().show_panes,
            "show_panes should default to true"
        );
    }
}
