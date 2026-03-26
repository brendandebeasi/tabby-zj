use crate::click::ClickTarget;
use crate::config::KeybindingsConfig;
use crate::persistence;
use crate::render::PINNED_HEIGHT;
use crate::state::{MenuState, MenuTarget, PluginState, RenameTarget};
use crate::workers;
use zellij_tile::prelude::*;

/// Logical actions that can be triggered by key presses.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    CursorUp,
    CursorDown,
    Activate,
    Dismiss,
    ToggleCollapse,
    NewTab,
}

pub fn handle_event(state: &mut PluginState, event: Event) -> bool {
    match event {
        Event::TabUpdate(tabs) => {
            state.active_tab_index = tabs.iter().position(|t| t.active).unwrap_or(0);
            state.tabs = tabs;
            state.rebuild_tab_entries();
            true
        }
        Event::PaneUpdate(manifest) => {
            state.pane_manifest = Some(manifest);
            state.rebuild_tab_entries();
            true
        }
        Event::Timer(_elapsed) => {
            state.tick_count += 1;
            for indicator in state.indicators.values_mut() {
                if indicator.busy {
                    indicator.busy_frame = indicator.busy_frame.wrapping_add(1);
                }
            }
            if state.tick_count % workers::GIT_POLL_INTERVAL == 1 {
                if let Some(cwd) = state.pane_cwds.values().next().cloned() {
                    workers::request_git_status(cwd);
                }
            }
            if state.config.widgets.stats.enabled {
                let stats_interval = state.config.widgets.stats.interval_secs.max(1);
                if state.tick_count % stats_interval == 1 {
                    if let Some(cwd) = state.pane_cwds.values().next().cloned() {
                        workers::request_stats(cwd);
                    }
                }
            }
            set_timeout(1.0);
            true
        }
        Event::CwdChanged(pane_id, cwd, _clients) => {
            if let PaneId::Terminal(id) = pane_id {
                state.pane_cwds.insert(id, cwd);
                state.rebuild_tab_entries();
            }
            true
        }
        Event::Mouse(mouse) => handle_mouse(state, mouse),
        Event::Key(key) => handle_key(state, key),
        Event::RunCommandResult(exit_code, stdout, _stderr, ctx) => {
            match ctx.get("type").map(String::as_str) {
                Some(workers::CTX_TYPE_GIT) => {
                    if exit_code == Some(0) {
                        let text = String::from_utf8_lossy(&stdout).into_owned();
                        state.git_status = Some(workers::parse_git_status(&text));
                    } else {
                        state.git_status = None;
                    }
                    true
                }
                Some(workers::CTX_TYPE_STATS) => {
                    if exit_code == Some(0) {
                        let text = String::from_utf8_lossy(&stdout).into_owned();
                        state.stats = Some(crate::widgets::stats::parse_stats_output(&text));
                    } else {
                        state.stats = None;
                    }
                    true
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn handle_mouse(state: &mut PluginState, mouse: Mouse) -> bool {
    match mouse {
        Mouse::ScrollUp(_) => {
            state.viewport_offset = state.viewport_offset.saturating_sub(3);
            true
        }
        Mouse::ScrollDown(_) => {
            state.viewport_offset = state.viewport_offset.saturating_add(3);
            true
        }
        Mouse::LeftClick(row, _col) => {
            if row < 0 {
                return false;
            }
            if state.sidebar_collapsed {
                state.sidebar_collapsed = false;
                flush_state(state);
                return true;
            }
            let logical = (row as usize) + state.viewport_offset;
            let target = state
                .click_regions
                .iter()
                .find(|r| r.line == logical)
                .map(|r| r.target.clone());
            dispatch_left_click(state, target)
        }
        Mouse::RightClick(row, _col) => {
            if row < 0 {
                return false;
            }
            let logical = (row as usize) + state.viewport_offset;
            let target = state
                .click_regions
                .iter()
                .find(|r| r.line == logical)
                .map(|r| r.target.clone());
            dispatch_right_click(state, target, row as usize)
        }
        _ => false,
    }
}

fn dispatch_left_click(state: &mut PluginState, target: Option<ClickTarget>) -> bool {
    match target {
        Some(ClickTarget::Tab(pos)) => {
            #[cfg(not(test))]
            switch_tab_to(pos as u32);
            let _ = pos;
            true
        }
        Some(ClickTarget::Group(name)) => {
            if state.collapsed_groups.contains(&name) {
                state.collapsed_groups.remove(&name);
            } else {
                state.collapsed_groups.insert(name);
            }
            flush_state(state);
            true
        }
        Some(ClickTarget::Pane(id)) => {
            #[cfg(not(test))]
            focus_terminal_pane(id, false, false);
            let _ = id;
            true
        }
        _ => false,
    }
}

fn dispatch_right_click(
    state: &mut PluginState,
    target: Option<ClickTarget>,
    visual_line: usize,
) -> bool {
    let menu_target = match target {
        Some(ClickTarget::Tab(pos)) => MenuTarget::Tab(pos),
        Some(ClickTarget::Group(name)) => MenuTarget::Group(name),
        Some(ClickTarget::Pane(id)) => MenuTarget::Pane(id),
        _ => return false,
    };
    let group_names: Vec<String> = state.config.groups.iter().map(|g| g.name.clone()).collect();
    let items = build_menu_items(&menu_target, &group_names);
    state.active_menu = Some(MenuState {
        target: menu_target,
        selected_index: 0,
        position_line: visual_line,
        parent_items: None,
        items_cache: Some(items),
    });
    true
}

fn parse_key_string(s: &str) -> Option<BareKey> {
    match s {
        "Enter" => Some(BareKey::Enter),
        "Esc" => Some(BareKey::Esc),
        "Up" => Some(BareKey::Up),
        "Down" => Some(BareKey::Down),
        "Left" => Some(BareKey::Left),
        "Right" => Some(BareKey::Right),
        "Backspace" => Some(BareKey::Backspace),
        s if s.chars().count() == 1 => Some(BareKey::Char(s.chars().next().unwrap())),
        _ => None,
    }
}

fn resolve_action(key: &KeyWithModifier, cfg: &KeybindingsConfig) -> Option<Action> {
    let bare = &key.bare_key;

    if let Some(s) = &cfg.cursor_down {
        if parse_key_string(s).as_ref() == Some(bare) {
            return Some(Action::CursorDown);
        }
    }
    if let Some(s) = &cfg.cursor_up {
        if parse_key_string(s).as_ref() == Some(bare) {
            return Some(Action::CursorUp);
        }
    }
    if let Some(s) = &cfg.activate {
        if parse_key_string(s).as_ref() == Some(bare) {
            return Some(Action::Activate);
        }
    }
    if let Some(s) = &cfg.dismiss {
        if parse_key_string(s).as_ref() == Some(bare) {
            return Some(Action::Dismiss);
        }
    }
    if let Some(s) = &cfg.toggle_collapse {
        if parse_key_string(s).as_ref() == Some(bare) {
            return Some(Action::ToggleCollapse);
        }
    }
    if let Some(s) = &cfg.new_tab {
        if parse_key_string(s).as_ref() == Some(bare) {
            return Some(Action::NewTab);
        }
    }

    match bare {
        BareKey::Up | BareKey::Char('k') if cfg.cursor_up.is_none() => Some(Action::CursorUp),
        BareKey::Down | BareKey::Char('j') if cfg.cursor_down.is_none() => Some(Action::CursorDown),
        BareKey::Enter if cfg.activate.is_none() => Some(Action::Activate),
        BareKey::Esc | BareKey::Char('q') if cfg.dismiss.is_none() => Some(Action::Dismiss),
        _ => None,
    }
}

fn flush_state(state: &PluginState) {
    persistence::save_state(&persistence::PersistedState {
        group_assignments: state.group_assignments.clone(),
        collapsed_groups: state.collapsed_groups.clone(),
        custom_colors: state.custom_colors.clone(),
        markers: state.markers.clone(),
        sidebar_collapsed: state.sidebar_collapsed,
    });
}

fn handle_key(state: &mut PluginState, key: KeyWithModifier) -> bool {
    if state.rename_state.is_some() {
        return handle_rename_key(state, key);
    }
    if state.active_menu.is_some() {
        return handle_menu_key(state, &key);
    }
    if state.active_picker.is_some() {
        return handle_picker_key(state, key);
    }
    if state.active_color_picker.is_some() {
        return handle_color_picker_key(state, key);
    }
    if !key.has_no_modifiers() {
        return false;
    }
    match resolve_action(&key, &state.config.keybindings) {
        Some(Action::CursorUp) => {
            cursor_move(state, -1);
            true
        }
        Some(Action::CursorDown) => {
            cursor_move(state, 1);
            true
        }
        Some(Action::Activate) => {
            cursor_activate(state);
            true
        }
        Some(Action::Dismiss) => {
            state.cursor_position = None;
            false
        }
        Some(Action::ToggleCollapse) | Some(Action::NewTab) | None => false,
    }
}

fn handle_rename_key(state: &mut PluginState, key: KeyWithModifier) -> bool {
    match key.bare_key {
        BareKey::Char(c) => {
            if let Some(rs) = &mut state.rename_state {
                rs.buffer.push(c);
            }
            true
        }
        BareKey::Backspace => {
            if let Some(rs) = &mut state.rename_state {
                rs.buffer.pop();
            }
            true
        }
        BareKey::Enter => {
            if let Some(rs) = state.rename_state.take() {
                commit_rename(state, rs.target, rs.buffer);
            }
            true
        }
        BareKey::Esc => {
            state.rename_state = None;
            true
        }
        _ => true,
    }
}

fn commit_rename(state: &mut PluginState, target: RenameTarget, new_name: String) {
    if new_name.trim().is_empty() {
        return;
    }
    match target {
        RenameTarget::Tab(pos) => {
            #[cfg(not(test))]
            rename_tab(pos as u32, &new_name);
            let _ = (pos, new_name);
        }
        RenameTarget::Pane(id) => {
            #[cfg(not(test))]
            rename_terminal_pane(id, &new_name);
            let _ = (id, new_name);
        }
        RenameTarget::Group(old_name) => {
            for val in state.group_assignments.values_mut() {
                if *val == old_name {
                    *val = new_name.clone();
                }
            }
            if state.collapsed_groups.remove(&old_name) {
                state.collapsed_groups.insert(new_name.clone());
            }
            flush_state(state);
        }
        RenameTarget::None => {}
    }
}

fn handle_menu_key(state: &mut PluginState, key: &KeyWithModifier) -> bool {
    if !key.has_no_modifiers() {
        return false;
    }
    match key.bare_key {
        BareKey::Up | BareKey::Char('k') => {
            if let Some(menu) = &mut state.active_menu {
                menu.selected_index = menu.selected_index.saturating_sub(1);
            }
            true
        }
        BareKey::Down | BareKey::Char('j') => {
            let count = menu_item_count(state);
            if let Some(menu) = &mut state.active_menu {
                menu.selected_index = (menu.selected_index + 1).min(count.saturating_sub(1));
            }
            true
        }
        BareKey::Enter => {
            if let Some(menu_state) = state.active_menu.clone() {
                let items = menu_state.items_cache.clone().unwrap_or_else(|| {
                    let group_names: Vec<String> =
                        state.config.groups.iter().map(|g| g.name.clone()).collect();
                    build_menu_items(&menu_state.target, &group_names)
                });
                if let Some(item) = items.get(menu_state.selected_index) {
                    if !item.is_separator {
                        match item.action.clone() {
                            crate::menus::MenuAction::Submenu(_, sub_items) => {
                                if let Some(menu) = &mut state.active_menu {
                                    if menu.parent_items.is_none() {
                                        menu.parent_items = Some(items.clone());
                                        menu.items_cache = Some(sub_items);
                                        menu.selected_index = 0;
                                    }
                                }
                            }
                            action => {
                                crate::menus::execute_action(state, action);
                            }
                        }
                    }
                }
            }
            true
        }
        BareKey::Esc => {
            let parent = state
                .active_menu
                .as_mut()
                .and_then(|m| m.parent_items.take());
            if let Some(parent_items) = parent {
                if let Some(menu) = &mut state.active_menu {
                    menu.items_cache = Some(parent_items);
                    menu.selected_index = 0;
                }
            } else {
                state.active_menu = None;
            }
            true
        }
        _ => false,
    }
}

fn handle_picker_key(state: &mut PluginState, key: KeyWithModifier) -> bool {
    if !key.has_no_modifiers() {
        return false;
    }
    let emojis_per_row = (state.cols / 3).max(1);
    match key.bare_key {
        BareKey::Right | BareKey::Char('l') => {
            if let Some(picker) = &mut state.active_picker {
                let max = picker.results.len().saturating_sub(1);
                picker.selected = (picker.selected + 1).min(max);
            }
            true
        }
        BareKey::Left | BareKey::Char('h') => {
            if let Some(picker) = &mut state.active_picker {
                picker.selected = picker.selected.saturating_sub(1);
            }
            true
        }
        BareKey::Down | BareKey::Char('j') => {
            if let Some(picker) = &mut state.active_picker {
                let max = picker.results.len().saturating_sub(1);
                picker.selected = (picker.selected + emojis_per_row).min(max);
            }
            true
        }
        BareKey::Up | BareKey::Char('k') => {
            if let Some(picker) = &mut state.active_picker {
                picker.selected = picker.selected.saturating_sub(emojis_per_row);
            }
            true
        }
        BareKey::Char(c) => {
            if let Some(picker) = &mut state.active_picker {
                picker.query.push(c);
                picker.filter();
            }
            true
        }
        BareKey::Backspace => {
            if let Some(picker) = &mut state.active_picker {
                picker.query.pop();
                picker.filter();
            }
            true
        }
        BareKey::Enter => {
            if let Some(picker) = state.active_picker.take() {
                if let Some(emoji) = picker.selected_emoji() {
                    crate::menus::execute_action(
                        state,
                        crate::menus::MenuAction::SetMarker(picker.target_tab, emoji.to_string()),
                    );
                }
            }
            true
        }
        BareKey::Esc => {
            state.active_picker = None;
            true
        }
        _ => true,
    }
}

fn handle_color_picker_key(state: &mut PluginState, key: KeyWithModifier) -> bool {
    if !key.has_no_modifiers() {
        return false;
    }
    match key.bare_key {
        BareKey::Down | BareKey::Char('j') => {
            if let Some(cp) = &mut state.active_color_picker {
                cp.next_component();
            }
            true
        }
        BareKey::Up | BareKey::Char('k') => {
            if let Some(cp) = &mut state.active_color_picker {
                cp.prev_component();
            }
            true
        }
        BareKey::Right | BareKey::Char('l') => {
            if let Some(cp) = &mut state.active_color_picker {
                cp.adjust(5.0);
            }
            true
        }
        BareKey::Left | BareKey::Char('h') => {
            if let Some(cp) = &mut state.active_color_picker {
                cp.adjust(-5.0);
            }
            true
        }
        BareKey::Enter => {
            if let Some(cp) = state.active_color_picker.take() {
                let hex = cp.current_hex();
                crate::menus::execute_action(
                    state,
                    crate::menus::MenuAction::SetColor(cp.target_tab, hex),
                );
            }
            true
        }
        BareKey::Esc => {
            state.active_color_picker = None;
            true
        }
        _ => true,
    }
}

fn build_menu_items(target: &MenuTarget, group_names: &[String]) -> Vec<crate::menus::MenuItem> {
    match target {
        MenuTarget::Tab(pos) => crate::menus::build_tab_menu(*pos, group_names),
        MenuTarget::Pane(id) => crate::menus::build_pane_menu(*id),
        MenuTarget::Group(name) => crate::menus::build_group_menu(name),
        MenuTarget::None => vec![],
    }
}

fn menu_item_count(state: &PluginState) -> usize {
    state
        .active_menu
        .as_ref()
        .map(|m| {
            if let Some(items) = &m.items_cache {
                items.len()
            } else {
                let group_names: Vec<String> =
                    state.config.groups.iter().map(|g| g.name.clone()).collect();
                build_menu_items(&m.target, &group_names).len()
            }
        })
        .unwrap_or(0)
}

fn cursor_move(state: &mut PluginState, delta: i32) {
    let total = state.click_regions.len();
    if total == 0 {
        state.cursor_position = None;
        return;
    }
    let current = state.cursor_position.unwrap_or(0);
    let new_pos = if delta < 0 {
        if current == 0 {
            total - 1
        } else {
            current - 1
        }
    } else {
        (current + 1) % total
    };
    state.cursor_position = Some(new_pos);
    cursor_ensure_visible(state);
}

fn cursor_activate(state: &mut PluginState) {
    if let Some(pos) = state.cursor_position {
        if let Some(region) = state.click_regions.get(pos) {
            let target = region.target.clone();
            let _ = dispatch_click(state, target);
        }
    }
}

fn dispatch_click(state: &mut PluginState, target: ClickTarget) -> bool {
    dispatch_left_click(state, Some(target))
}

fn cursor_ensure_visible(state: &mut PluginState) {
    let pos = match state.cursor_position {
        Some(p) => p,
        None => return,
    };
    // 2 lines reserved for pinned widget area at bottom
    let scrollable_rows = state.rows.saturating_sub(PINNED_HEIGHT);
    if scrollable_rows == 0 {
        return;
    }
    if pos < state.viewport_offset {
        state.viewport_offset = pos;
    } else if pos >= state.viewport_offset + scrollable_rows {
        state.viewport_offset = pos.saturating_sub(scrollable_rows - 1);
    }
}

/// Structured representation of a pipe command payload.
#[derive(Debug, PartialEq)]
pub enum PipeCommand {
    SetIndicator {
        indicator: String,
        value: bool,
        pane_id: Option<String>,
    },
    Collapse(bool),
    Toggle,
    ReloadConfig,
    SetMarker {
        tab_key: String,
        emoji: String,
    },
    SetQuota {
        data: String,
    },
    Unknown(String),
}

/// Parse a raw pipe payload string + args map into a `PipeCommand`.
///
/// Wire formats (unchanged):
/// - `"busy:1"` / `"busy:0"` / `"busy:true"` / `"busy:on"`
/// - `"bell:1"` / `"bell:0"`
/// - `"input:1"` / `"input:0"`
/// - `"collapse:1"` / `"collapse:0"`
/// - `"toggle:<any>"` — value ignored, always toggles
/// - `"config:<any>"` — value ignored, always reloads
/// - `"marker:<tab_key>:<emoji>"`
/// - `"quota:<data>"`
/// - anything else → `Unknown`
pub fn parse_pipe(payload: &str, args: &std::collections::BTreeMap<String, String>) -> PipeCommand {
    let parts: Vec<&str> = payload.splitn(3, ':').collect();
    if parts.is_empty() || (parts.len() == 1 && parts[0].is_empty()) {
        return PipeCommand::Unknown(payload.to_string());
    }

    let cmd = parts[0];

    match cmd {
        "toggle" => return PipeCommand::Toggle,
        "config" => return PipeCommand::ReloadConfig,
        _ => {}
    }

    // All remaining commands require at least a second part.
    if parts.len() < 2 {
        return PipeCommand::Unknown(payload.to_string());
    }

    let rest = parts[1];

    match cmd {
        "collapse" => {
            let value = rest == "1" || rest == "true" || rest == "on";
            PipeCommand::Collapse(value)
        }
        "quota" => {
            // rest is everything after the first colon
            let data = if parts.len() > 2 {
                format!("{}:{}", rest, parts[2])
            } else {
                rest.to_string()
            };
            PipeCommand::SetQuota { data }
        }
        "marker" => {
            if parts.len() < 3 {
                return PipeCommand::Unknown(payload.to_string());
            }
            PipeCommand::SetMarker {
                tab_key: rest.to_string(),
                emoji: parts[2].to_string(),
            }
        }
        "busy" | "bell" | "input" => {
            let value = rest == "1" || rest == "true" || rest == "on";
            // pane_id: prefer args map, then fall back to third colon-segment
            let pane_id = args.get("pane_id").cloned().or_else(|| {
                if parts.len() > 2 {
                    Some(parts[2].to_string())
                } else {
                    None
                }
            });
            PipeCommand::SetIndicator {
                indicator: cmd.to_string(),
                value,
                pane_id,
            }
        }
        _ => PipeCommand::Unknown(payload.to_string()),
    }
}

pub fn handle_pipe(state: &mut PluginState, pipe_message: PipeMessage) -> bool {
    let payload = match &pipe_message.payload {
        Some(p) => p.clone(),
        None => return false,
    };

    match parse_pipe(&payload, &pipe_message.args) {
        PipeCommand::Collapse(value) => {
            state.sidebar_collapsed = value;
            flush_state(state);
            true
        }
        PipeCommand::Toggle => {
            state.sidebar_collapsed = !state.sidebar_collapsed;
            flush_state(state);
            true
        }
        PipeCommand::ReloadConfig => {
            state.config = crate::config::Config::load();
            true
        }
        PipeCommand::SetIndicator {
            indicator,
            value,
            pane_id,
        } => {
            let id = pane_id.unwrap_or_else(|| "default".to_string());
            let entry = state.indicators.entry(id).or_default();
            match indicator.as_str() {
                "busy" => entry.busy = value,
                "bell" => entry.bell = value,
                "input" => entry.input = value,
                _ => return false,
            }
            true
        }
        PipeCommand::SetMarker { .. } => {
            // Marker handling not yet wired to state — return true to ack receipt
            true
        }
        PipeCommand::SetQuota { .. } => {
            // Quota handling not yet wired to state — return true to ack receipt
            true
        }
        PipeCommand::Unknown(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::click::{ClickRegion, ClickTarget};
    use crate::state::{MenuTarget, PluginState, TabKey};

    fn make_pipe(payload: &str) -> PipeMessage {
        PipeMessage {
            source: PipeSource::Cli("test".into()),
            name: "tabby".into(),
            payload: Some(payload.into()),
            args: std::collections::BTreeMap::new(),
            is_private: false,
        }
    }

    fn make_pipe_with_pane(payload: &str, pane_id: &str) -> PipeMessage {
        let mut args = std::collections::BTreeMap::new();
        args.insert("pane_id".into(), pane_id.into());
        PipeMessage {
            source: PipeSource::Cli("test".into()),
            name: "tabby".into(),
            payload: Some(payload.into()),
            args,
            is_private: false,
        }
    }

    fn region(line: usize, target: ClickTarget) -> ClickRegion {
        ClickRegion { line, target }
    }

    fn sim_left(state: &mut PluginState, line: usize) -> bool {
        handle_mouse(state, Mouse::LeftClick(line as isize, 0))
    }

    fn sim_right(state: &mut PluginState, line: usize) -> bool {
        handle_mouse(state, Mouse::RightClick(line as isize, 0))
    }

    fn make_key(bare: BareKey) -> KeyWithModifier {
        KeyWithModifier::new(bare)
    }

    fn three_regions() -> Vec<ClickRegion> {
        vec![
            region(0, ClickTarget::Tab(0)),
            region(1, ClickTarget::Tab(1)),
            region(2, ClickTarget::Tab(2)),
        ]
    }

    #[test]
    fn test_key_down_increments_cursor() {
        let mut state = PluginState::default();
        state.click_regions = three_regions();
        state.cursor_position = Some(0);
        handle_key(&mut state, make_key(BareKey::Down));
        assert_eq!(state.cursor_position, Some(1));
    }

    #[test]
    fn test_key_up_decrements_cursor() {
        let mut state = PluginState::default();
        state.click_regions = three_regions();
        state.cursor_position = Some(2);
        handle_key(&mut state, make_key(BareKey::Up));
        assert_eq!(state.cursor_position, Some(1));
    }

    #[test]
    fn test_key_down_wraps_at_end() {
        let mut state = PluginState::default();
        state.click_regions = three_regions();
        state.cursor_position = Some(2);
        handle_key(&mut state, make_key(BareKey::Down));
        assert_eq!(state.cursor_position, Some(0));
    }

    #[test]
    fn test_key_up_wraps_at_start() {
        let mut state = PluginState::default();
        state.click_regions = three_regions();
        state.cursor_position = Some(0);
        handle_key(&mut state, make_key(BareKey::Up));
        assert_eq!(state.cursor_position, Some(2));
    }

    #[test]
    fn test_key_j_same_as_down() {
        let mut state = PluginState::default();
        state.click_regions = three_regions();
        state.cursor_position = Some(0);
        handle_key(&mut state, make_key(BareKey::Char('j')));
        assert_eq!(state.cursor_position, Some(1));
    }

    #[test]
    fn test_key_k_same_as_up() {
        let mut state = PluginState::default();
        state.click_regions = three_regions();
        state.cursor_position = Some(2);
        handle_key(&mut state, make_key(BareKey::Char('k')));
        assert_eq!(state.cursor_position, Some(1));
    }

    #[test]
    fn test_key_enter_dispatches_group_toggle() {
        let mut state = PluginState::default();
        state.click_regions = vec![region(0, ClickTarget::Group("Default".into()))];
        state.cursor_position = Some(0);
        let result = handle_key(&mut state, make_key(BareKey::Enter));
        assert!(result);
        assert!(state.collapsed_groups.contains("Default"));
    }

    #[test]
    fn test_key_esc_clears_cursor() {
        let mut state = PluginState::default();
        state.click_regions = three_regions();
        state.cursor_position = Some(1);
        let result = handle_key(&mut state, make_key(BareKey::Esc));
        assert!(!result, "Esc should return false so Zellij handles focus");
        assert_eq!(state.cursor_position, None);
    }

    #[test]
    fn test_esc_dismisses_menu() {
        let mut state = PluginState::default();
        state.active_menu = Some(MenuState {
            target: MenuTarget::Tab(0),
            selected_index: 0,
            position_line: 0,
            ..Default::default()
        });
        let result = handle_key(&mut state, make_key(BareKey::Esc));
        assert!(result, "Esc on open menu should return true");
        assert!(state.active_menu.is_none());
    }

    #[test]
    fn test_cursor_auto_scrolls_down() {
        let mut state = PluginState::default();
        state.click_regions = (0..9).map(|i| region(i, ClickTarget::Tab(i))).collect();
        state.cursor_position = Some(7);
        state.viewport_offset = 0;
        state.rows = 6;
        handle_key(&mut state, make_key(BareKey::Down));
        assert_eq!(state.cursor_position, Some(8));
        assert_eq!(state.viewport_offset, 5);
    }

    #[test]
    fn test_scroll_up_decrements_offset() {
        let mut state = PluginState::default();
        state.viewport_offset = 6;
        handle_mouse(&mut state, Mouse::ScrollUp(0));
        assert_eq!(state.viewport_offset, 3);
    }

    #[test]
    fn test_scroll_up_clamps_at_zero() {
        let mut state = PluginState::default();
        state.viewport_offset = 0;
        handle_mouse(&mut state, Mouse::ScrollUp(0));
        assert_eq!(state.viewport_offset, 0);
    }

    #[test]
    fn test_scroll_up_partial_clamp() {
        let mut state = PluginState::default();
        state.viewport_offset = 1;
        handle_mouse(&mut state, Mouse::ScrollUp(0));
        assert_eq!(state.viewport_offset, 0);
    }

    #[test]
    fn test_scroll_down_increments_offset() {
        let mut state = PluginState::default();
        handle_mouse(&mut state, Mouse::ScrollDown(0));
        assert_eq!(state.viewport_offset, 3);
    }

    #[test]
    fn test_scroll_down_additive() {
        let mut state = PluginState::default();
        state.viewport_offset = 3;
        handle_mouse(&mut state, Mouse::ScrollDown(0));
        assert_eq!(state.viewport_offset, 6);
    }

    #[test]
    fn test_left_click_on_tab_returns_true() {
        let mut state = PluginState::default();
        state.click_regions = vec![region(2, ClickTarget::Tab(5))];
        assert!(sim_left(&mut state, 2));
    }

    #[test]
    fn test_left_click_on_group_collapses() {
        let mut state = PluginState::default();
        state.click_regions = vec![region(0, ClickTarget::Group("Default".into()))];
        assert!(sim_left(&mut state, 0));
        assert!(state.collapsed_groups.contains("Default"));
    }

    #[test]
    fn test_left_click_on_group_expands_if_collapsed() {
        let mut state = PluginState::default();
        state.collapsed_groups.insert("Default".into());
        state.click_regions = vec![region(0, ClickTarget::Group("Default".into()))];
        sim_left(&mut state, 0);
        assert!(!state.collapsed_groups.contains("Default"));
    }

    #[test]
    fn test_left_click_on_pane_returns_true() {
        let mut state = PluginState::default();
        state.click_regions = vec![region(1, ClickTarget::Pane(42))];
        assert!(sim_left(&mut state, 1));
    }

    #[test]
    fn test_left_click_off_region_returns_false() {
        let mut state = PluginState::default();
        state.click_regions = vec![];
        assert!(!sim_left(&mut state, 999));
    }

    #[test]
    fn test_left_click_empty_target_returns_false() {
        let mut state = PluginState::default();
        state.click_regions = vec![region(0, ClickTarget::Empty)];
        assert!(!sim_left(&mut state, 0));
    }

    #[test]
    fn test_left_click_uses_viewport_offset() {
        let mut state = PluginState::default();
        state.viewport_offset = 1;
        state.click_regions = vec![
            region(0, ClickTarget::Group("First".into())),
            region(1, ClickTarget::Group("Second".into())),
        ];
        sim_left(&mut state, 0);
        assert!(
            state.collapsed_groups.contains("Second"),
            "visual row 0 + offset 1 should hit logical line 1 = Second"
        );
    }

    #[test]
    fn test_right_click_on_tab_sets_menu() {
        let mut state = PluginState::default();
        state.click_regions = vec![region(2, ClickTarget::Tab(5))];
        assert!(sim_right(&mut state, 2));
        let menu = state.active_menu.as_ref().expect("menu should be set");
        assert!(matches!(menu.target, MenuTarget::Tab(5)));
        assert_eq!(menu.position_line, 2);
        assert_eq!(menu.selected_index, 0);
    }

    #[test]
    fn test_right_click_on_group_sets_group_menu() {
        let mut state = PluginState::default();
        state.click_regions = vec![region(0, ClickTarget::Group("Backend".into()))];
        assert!(sim_right(&mut state, 0));
        let menu = state.active_menu.as_ref().expect("menu should be set");
        assert!(matches!(&menu.target, MenuTarget::Group(n) if n == "Backend"));
    }

    #[test]
    fn test_right_click_on_pane_sets_pane_menu() {
        let mut state = PluginState::default();
        state.click_regions = vec![region(3, ClickTarget::Pane(42))];
        assert!(sim_right(&mut state, 3));
        let menu = state.active_menu.as_ref().expect("menu should be set");
        assert!(matches!(menu.target, MenuTarget::Pane(42)));
    }

    #[test]
    fn test_right_click_off_region_returns_false_no_menu() {
        let mut state = PluginState::default();
        state.click_regions = vec![];
        assert!(!sim_right(&mut state, 5));
        assert!(state.active_menu.is_none());
    }

    #[test]
    fn test_negative_click_row_returns_false() {
        let mut state = PluginState::default();
        state.click_regions = vec![region(0, ClickTarget::Tab(0))];
        assert!(!handle_mouse(&mut state, Mouse::LeftClick(-1, 0)));
        assert!(!handle_mouse(&mut state, Mouse::RightClick(-1, 0)));
    }

    #[test]
    fn test_pipe_busy_on() {
        let mut state = PluginState::default();
        assert!(handle_pipe(&mut state, make_pipe("busy:1")));
        assert!(state.indicators["default"].busy);
    }

    #[test]
    fn test_pipe_busy_off() {
        let mut state = PluginState::default();
        state.indicators.entry("default".into()).or_default().busy = true;
        handle_pipe(&mut state, make_pipe("busy:0"));
        assert!(!state.indicators["default"].busy);
    }

    #[test]
    fn test_pipe_bell() {
        let mut state = PluginState::default();
        handle_pipe(&mut state, make_pipe("bell:1"));
        assert!(state.indicators["default"].bell);
    }

    #[test]
    fn test_pipe_input() {
        let mut state = PluginState::default();
        handle_pipe(&mut state, make_pipe("input:1"));
        assert!(state.indicators["default"].input);
    }

    #[test]
    fn test_pipe_with_explicit_pane_id() {
        let mut state = PluginState::default();
        handle_pipe(&mut state, make_pipe_with_pane("busy:1", "%42"));
        assert!(state.indicators["%42"].busy);
    }

    #[test]
    fn test_pipe_unknown_type_returns_false() {
        let mut state = PluginState::default();
        assert!(!handle_pipe(&mut state, make_pipe("unknown:1")));
    }

    #[test]
    fn test_pipe_no_payload_returns_false() {
        let mut state = PluginState::default();
        let msg = PipeMessage {
            source: PipeSource::Cli("test".into()),
            name: "tabby".into(),
            payload: None,
            args: std::collections::BTreeMap::new(),
            is_private: false,
        };
        assert!(!handle_pipe(&mut state, msg));
    }

    #[test]
    fn test_pipe_malformed_payload_returns_false() {
        let mut state = PluginState::default();
        assert!(!handle_pipe(&mut state, make_pipe("nocolon")));
    }

    fn make_rename_state(
        target: crate::state::RenameTarget,
        buf: &str,
    ) -> crate::state::RenameState {
        crate::state::RenameState {
            target,
            buffer: buf.into(),
        }
    }

    #[test]
    fn test_rename_char_appends_to_buffer() {
        let mut state = PluginState::default();
        state.rename_state = Some(make_rename_state(RenameTarget::Tab(0), "ap"));
        handle_key(&mut state, make_key(BareKey::Char('i')));
        assert_eq!(state.rename_state.as_ref().unwrap().buffer, "api");
    }

    #[test]
    fn test_rename_backspace_removes_last_char() {
        let mut state = PluginState::default();
        state.rename_state = Some(make_rename_state(RenameTarget::Tab(0), "api"));
        handle_key(&mut state, make_key(BareKey::Backspace));
        assert_eq!(state.rename_state.as_ref().unwrap().buffer, "ap");
    }

    #[test]
    fn test_rename_backspace_on_empty_is_noop() {
        let mut state = PluginState::default();
        state.rename_state = Some(make_rename_state(RenameTarget::Tab(0), ""));
        handle_key(&mut state, make_key(BareKey::Backspace));
        assert_eq!(state.rename_state.as_ref().unwrap().buffer, "");
    }

    #[test]
    fn test_rename_esc_cancels_clears_state() {
        let mut state = PluginState::default();
        state.rename_state = Some(make_rename_state(RenameTarget::Tab(0), "foo"));
        let result = handle_key(&mut state, make_key(BareKey::Esc));
        assert!(result);
        assert!(state.rename_state.is_none());
    }

    #[test]
    fn test_rename_enter_clears_rename_state() {
        let mut state = PluginState::default();
        state.rename_state = Some(make_rename_state(RenameTarget::Tab(0), "new-name"));
        handle_key(&mut state, make_key(BareKey::Enter));
        assert!(state.rename_state.is_none());
    }

    #[test]
    fn test_rename_group_updates_assignments() {
        let mut state = PluginState::default();
        state
            .group_assignments
            .insert(TabKey::new("api", 0), "OldName".into());
        state.rename_state = Some(make_rename_state(
            RenameTarget::Group("OldName".into()),
            "NewName",
        ));
        handle_key(&mut state, make_key(BareKey::Enter));
        assert_eq!(
            state.group_assignments.get(&TabKey::new("api", 0)),
            Some(&"NewName".into())
        );
    }

    #[test]
    fn test_rename_group_updates_collapsed() {
        let mut state = PluginState::default();
        state.collapsed_groups.insert("OldName".into());
        state.rename_state = Some(make_rename_state(
            RenameTarget::Group("OldName".into()),
            "NewName",
        ));
        handle_key(&mut state, make_key(BareKey::Enter));
        assert!(!state.collapsed_groups.contains("OldName"));
        assert!(state.collapsed_groups.contains("NewName"));
    }

    #[test]
    fn test_rename_empty_buffer_does_not_commit() {
        let mut state = PluginState::default();
        state
            .group_assignments
            .insert(TabKey::new("api", 0), "MyGroup".into());
        state.rename_state = Some(make_rename_state(RenameTarget::Group("MyGroup".into()), ""));
        handle_key(&mut state, make_key(BareKey::Enter));
        assert_eq!(
            state.group_assignments.get(&TabKey::new("api", 0)),
            Some(&"MyGroup".into())
        );
    }

    #[test]
    fn test_rename_blocks_menu_keys() {
        let mut state = PluginState::default();
        state.rename_state = Some(make_rename_state(RenameTarget::Tab(0), "foo"));
        state.active_menu = Some(crate::state::MenuState {
            target: MenuTarget::Tab(0),
            selected_index: 0,
            position_line: 0,
            ..Default::default()
        });
        handle_key(&mut state, make_key(BareKey::Esc));
        assert!(
            state.rename_state.is_none(),
            "rename Esc should clear rename, not menu"
        );
        assert!(state.active_menu.is_some(), "menu should still be open");
    }

    #[test]
    fn test_menu_up_decrements_selected_index() {
        let mut state = PluginState::default();
        state.active_menu = Some(crate::state::MenuState {
            target: MenuTarget::Tab(0),
            selected_index: 2,
            position_line: 0,
            ..Default::default()
        });
        handle_key(&mut state, make_key(BareKey::Up));
        assert_eq!(state.active_menu.as_ref().unwrap().selected_index, 1);
    }

    #[test]
    fn test_menu_down_increments_selected_index() {
        let mut state = PluginState::default();
        state.active_menu = Some(crate::state::MenuState {
            target: MenuTarget::Tab(0),
            selected_index: 0,
            position_line: 0,
            ..Default::default()
        });
        handle_key(&mut state, make_key(BareKey::Down));
        assert_eq!(state.active_menu.as_ref().unwrap().selected_index, 1);
    }

    #[test]
    fn test_pipe_collapse_on() {
        let mut state = PluginState::default();
        state.sidebar_collapsed = false;
        assert!(handle_pipe(&mut state, make_pipe("collapse:1")));
        assert!(state.sidebar_collapsed);
    }

    #[test]
    fn test_pipe_collapse_off() {
        let mut state = PluginState::default();
        state.sidebar_collapsed = true;
        assert!(handle_pipe(&mut state, make_pipe("collapse:0")));
        assert!(!state.sidebar_collapsed);
    }

    #[test]
    fn test_pipe_toggle_flips_false_to_true() {
        let mut state = PluginState::default();
        state.sidebar_collapsed = false;
        assert!(handle_pipe(&mut state, make_pipe("toggle:1")));
        assert!(state.sidebar_collapsed);
    }

    #[test]
    fn test_pipe_toggle_flips_true_to_false() {
        let mut state = PluginState::default();
        state.sidebar_collapsed = true;
        assert!(handle_pipe(&mut state, make_pipe("toggle:1")));
        assert!(!state.sidebar_collapsed);
    }

    #[test]
    fn test_click_on_collapsed_sidebar_expands() {
        let mut state = PluginState::default();
        state.sidebar_collapsed = true;
        state.click_regions = vec![];
        let result = handle_mouse(&mut state, Mouse::LeftClick(5, 0));
        assert!(result);
        assert!(!state.sidebar_collapsed);
    }

    #[test]
    fn test_click_on_normal_sidebar_does_not_change_collapsed() {
        let mut state = PluginState::default();
        state.sidebar_collapsed = false;
        state.click_regions = vec![region(0, ClickTarget::Tab(0))];
        handle_mouse(&mut state, Mouse::LeftClick(0, 0));
        assert!(!state.sidebar_collapsed);
    }

    #[test]
    fn test_pipe_collapse_still_handles_busy() {
        let mut state = PluginState::default();
        assert!(handle_pipe(&mut state, make_pipe("collapse:1")));
        assert!(state.sidebar_collapsed);
        assert!(handle_pipe(&mut state, make_pipe("busy:1")));
        assert!(state.indicators["default"].busy);
    }

    #[test]
    fn test_pipe_collapse_does_not_affect_indicators() {
        let mut state = PluginState::default();
        state.indicators.entry("default".into()).or_default().busy = true;
        handle_pipe(&mut state, make_pipe("collapse:1"));
        assert!(
            state.indicators["default"].busy,
            "collapse should not touch indicators"
        );
    }

    #[test]
    fn test_pipe_config_reload_returns_true() {
        let mut state = PluginState::default();
        let result = handle_pipe(&mut state, make_pipe("config:reload"));
        assert!(result, "config:reload should return true");
    }

    #[test]
    fn test_pipe_config_any_value_reloads() {
        let mut state = PluginState::default();
        let result = handle_pipe(&mut state, make_pipe("config:1"));
        assert!(result);
    }

    fn empty_args() -> std::collections::BTreeMap<String, String> {
        std::collections::BTreeMap::new()
    }

    fn args_with_pane(pane_id: &str) -> std::collections::BTreeMap<String, String> {
        let mut m = std::collections::BTreeMap::new();
        m.insert("pane_id".into(), pane_id.into());
        m
    }

    #[test]
    fn test_parse_pipe_busy_on() {
        assert_eq!(
            parse_pipe("busy:1", &empty_args()),
            PipeCommand::SetIndicator {
                indicator: "busy".into(),
                value: true,
                pane_id: None,
            }
        );
    }

    #[test]
    fn test_parse_pipe_busy_off() {
        assert_eq!(
            parse_pipe("busy:0", &empty_args()),
            PipeCommand::SetIndicator {
                indicator: "busy".into(),
                value: false,
                pane_id: None,
            }
        );
    }

    #[test]
    fn test_parse_pipe_busy_true_variant() {
        assert_eq!(
            parse_pipe("busy:true", &empty_args()),
            PipeCommand::SetIndicator {
                indicator: "busy".into(),
                value: true,
                pane_id: None,
            }
        );
    }

    #[test]
    fn test_parse_pipe_bell_on() {
        assert_eq!(
            parse_pipe("bell:1", &empty_args()),
            PipeCommand::SetIndicator {
                indicator: "bell".into(),
                value: true,
                pane_id: None,
            }
        );
    }

    #[test]
    fn test_parse_pipe_input_on() {
        assert_eq!(
            parse_pipe("input:1", &empty_args()),
            PipeCommand::SetIndicator {
                indicator: "input".into(),
                value: true,
                pane_id: None,
            }
        );
    }

    #[test]
    fn test_parse_pipe_indicator_pane_id_from_args() {
        assert_eq!(
            parse_pipe("busy:1", &args_with_pane("%42")),
            PipeCommand::SetIndicator {
                indicator: "busy".into(),
                value: true,
                pane_id: Some("%42".into()),
            }
        );
    }

    #[test]
    fn test_parse_pipe_indicator_pane_id_from_third_segment() {
        assert_eq!(
            parse_pipe("busy:1:%99", &empty_args()),
            PipeCommand::SetIndicator {
                indicator: "busy".into(),
                value: true,
                pane_id: Some("%99".into()),
            }
        );
    }

    #[test]
    fn test_parse_pipe_indicator_args_pane_id_wins_over_segment() {
        assert_eq!(
            parse_pipe("busy:1:%99", &args_with_pane("%42")),
            PipeCommand::SetIndicator {
                indicator: "busy".into(),
                value: true,
                pane_id: Some("%42".into()),
            }
        );
    }

    #[test]
    fn test_parse_pipe_collapse_on() {
        assert_eq!(
            parse_pipe("collapse:1", &empty_args()),
            PipeCommand::Collapse(true)
        );
    }

    #[test]
    fn test_parse_pipe_collapse_off() {
        assert_eq!(
            parse_pipe("collapse:0", &empty_args()),
            PipeCommand::Collapse(false)
        );
    }

    #[test]
    fn test_parse_pipe_toggle() {
        assert_eq!(parse_pipe("toggle:1", &empty_args()), PipeCommand::Toggle);
    }

    #[test]
    fn test_parse_pipe_toggle_no_value() {
        assert_eq!(parse_pipe("toggle", &empty_args()), PipeCommand::Toggle);
    }

    #[test]
    fn test_parse_pipe_config_reload() {
        assert_eq!(
            parse_pipe("config:reload", &empty_args()),
            PipeCommand::ReloadConfig
        );
    }

    #[test]
    fn test_parse_pipe_config_any_value() {
        assert_eq!(
            parse_pipe("config:1", &empty_args()),
            PipeCommand::ReloadConfig
        );
    }

    #[test]
    fn test_parse_pipe_config_no_value() {
        assert_eq!(
            parse_pipe("config", &empty_args()),
            PipeCommand::ReloadConfig
        );
    }

    #[test]
    fn test_parse_pipe_quota() {
        assert_eq!(
            parse_pipe("quota:somedata", &empty_args()),
            PipeCommand::SetQuota {
                data: "somedata".into()
            }
        );
    }

    #[test]
    fn test_parse_pipe_quota_with_colons() {
        assert_eq!(
            parse_pipe("quota:a:b", &empty_args()),
            PipeCommand::SetQuota { data: "a:b".into() }
        );
    }

    #[test]
    fn test_parse_pipe_marker() {
        assert_eq!(
            parse_pipe("marker:mykey:🔥", &empty_args()),
            PipeCommand::SetMarker {
                tab_key: "mykey".into(),
                emoji: "🔥".into(),
            }
        );
    }

    #[test]
    fn test_parse_pipe_marker_missing_emoji_is_unknown() {
        assert_eq!(
            parse_pipe("marker:mykey", &empty_args()),
            PipeCommand::Unknown("marker:mykey".into())
        );
    }

    #[test]
    fn test_parse_pipe_empty_string_is_unknown() {
        assert_eq!(
            parse_pipe("", &empty_args()),
            PipeCommand::Unknown("".into())
        );
    }

    #[test]
    fn test_parse_pipe_no_colon_is_unknown() {
        assert_eq!(
            parse_pipe("nocolon", &empty_args()),
            PipeCommand::Unknown("nocolon".into())
        );
    }

    #[test]
    fn test_parse_pipe_unknown_command() {
        assert_eq!(
            parse_pipe("unknown:1", &empty_args()),
            PipeCommand::Unknown("unknown:1".into())
        );
    }

    #[test]
    fn test_parse_pipe_busy_not_a_bool_is_false_value() {
        assert_eq!(
            parse_pipe("busy:notabool", &empty_args()),
            PipeCommand::SetIndicator {
                indicator: "busy".into(),
                value: false,
                pane_id: None,
            }
        );
    }

    fn default_cfg() -> crate::config::KeybindingsConfig {
        crate::config::KeybindingsConfig::default()
    }

    #[test]
    fn test_resolve_action_default_j_is_cursor_down() {
        let action = resolve_action(&make_key(BareKey::Char('j')), &default_cfg());
        assert_eq!(action, Some(Action::CursorDown));
    }

    #[test]
    fn test_resolve_action_default_down_is_cursor_down() {
        let action = resolve_action(&make_key(BareKey::Down), &default_cfg());
        assert_eq!(action, Some(Action::CursorDown));
    }

    #[test]
    fn test_resolve_action_default_k_is_cursor_up() {
        let action = resolve_action(&make_key(BareKey::Char('k')), &default_cfg());
        assert_eq!(action, Some(Action::CursorUp));
    }

    #[test]
    fn test_resolve_action_default_up_is_cursor_up() {
        let action = resolve_action(&make_key(BareKey::Up), &default_cfg());
        assert_eq!(action, Some(Action::CursorUp));
    }

    #[test]
    fn test_resolve_action_default_enter_is_activate() {
        let action = resolve_action(&make_key(BareKey::Enter), &default_cfg());
        assert_eq!(action, Some(Action::Activate));
    }

    #[test]
    fn test_resolve_action_default_esc_is_dismiss() {
        let action = resolve_action(&make_key(BareKey::Esc), &default_cfg());
        assert_eq!(action, Some(Action::Dismiss));
    }

    #[test]
    fn test_resolve_action_default_q_is_dismiss() {
        let action = resolve_action(&make_key(BareKey::Char('q')), &default_cfg());
        assert_eq!(action, Some(Action::Dismiss));
    }

    #[test]
    fn test_resolve_action_unknown_key_is_none() {
        let action = resolve_action(&make_key(BareKey::Char('x')), &default_cfg());
        assert_eq!(action, None);
    }

    #[test]
    fn test_resolve_action_custom_cursor_down_to_n() {
        let cfg = crate::config::KeybindingsConfig {
            cursor_down: Some("n".into()),
            ..Default::default()
        };
        assert_eq!(
            resolve_action(&make_key(BareKey::Char('n')), &cfg),
            Some(Action::CursorDown)
        );
    }

    #[test]
    fn test_resolve_action_custom_cursor_down_suppresses_default_j() {
        let cfg = crate::config::KeybindingsConfig {
            cursor_down: Some("n".into()),
            ..Default::default()
        };
        assert_eq!(resolve_action(&make_key(BareKey::Char('j')), &cfg), None);
        assert_eq!(resolve_action(&make_key(BareKey::Down), &cfg), None);
    }

    #[test]
    fn test_resolve_action_custom_cursor_up_suppresses_default_k() {
        let cfg = crate::config::KeybindingsConfig {
            cursor_up: Some("p".into()),
            ..Default::default()
        };
        assert_eq!(
            resolve_action(&make_key(BareKey::Char('p')), &cfg),
            Some(Action::CursorUp)
        );
        assert_eq!(resolve_action(&make_key(BareKey::Char('k')), &cfg), None);
        assert_eq!(resolve_action(&make_key(BareKey::Up), &cfg), None);
    }

    #[test]
    fn test_resolve_action_custom_dismiss_to_d() {
        let cfg = crate::config::KeybindingsConfig {
            dismiss: Some("d".into()),
            ..Default::default()
        };
        assert_eq!(
            resolve_action(&make_key(BareKey::Char('d')), &cfg),
            Some(Action::Dismiss)
        );
        assert_eq!(resolve_action(&make_key(BareKey::Esc), &cfg), None);
        assert_eq!(resolve_action(&make_key(BareKey::Char('q')), &cfg), None);
    }

    #[test]
    fn test_resolve_action_enter_key_string() {
        let cfg = crate::config::KeybindingsConfig {
            activate: Some("Enter".into()),
            ..Default::default()
        };
        assert_eq!(
            resolve_action(&make_key(BareKey::Enter), &cfg),
            Some(Action::Activate)
        );
    }

    #[test]
    fn test_custom_binding_cursor_down_moves_cursor() {
        let mut state = PluginState::default();
        state.config.keybindings.cursor_down = Some("n".into());
        state.click_regions = three_regions();
        state.cursor_position = Some(0);
        handle_key(&mut state, make_key(BareKey::Char('n')));
        assert_eq!(state.cursor_position, Some(1));
    }

    #[test]
    fn test_custom_binding_j_no_longer_moves_cursor_down() {
        let mut state = PluginState::default();
        state.config.keybindings.cursor_down = Some("n".into());
        state.click_regions = three_regions();
        state.cursor_position = Some(0);
        handle_key(&mut state, make_key(BareKey::Char('j')));
        assert_eq!(
            state.cursor_position,
            Some(0),
            "'j' should not move cursor when remapped"
        );
    }

    #[test]
    fn test_custom_binding_cursor_up_moves_cursor() {
        let mut state = PluginState::default();
        state.config.keybindings.cursor_up = Some("p".into());
        state.click_regions = three_regions();
        state.cursor_position = Some(2);
        handle_key(&mut state, make_key(BareKey::Char('p')));
        assert_eq!(state.cursor_position, Some(1));
    }

    #[test]
    fn test_unbound_actions_still_use_defaults_when_others_remapped() {
        let mut state = PluginState::default();
        state.config.keybindings.cursor_down = Some("n".into());
        state.click_regions = three_regions();
        state.cursor_position = Some(2);
        handle_key(&mut state, make_key(BareKey::Char('k')));
        assert_eq!(
            state.cursor_position,
            Some(1),
            "'k' should still work for cursor_up"
        );
    }

    #[test]
    fn test_parse_key_string_enter() {
        assert_eq!(parse_key_string("Enter"), Some(BareKey::Enter));
    }

    #[test]
    fn test_parse_key_string_esc() {
        assert_eq!(parse_key_string("Esc"), Some(BareKey::Esc));
    }

    #[test]
    fn test_parse_key_string_single_char() {
        assert_eq!(parse_key_string("n"), Some(BareKey::Char('n')));
        assert_eq!(parse_key_string("j"), Some(BareKey::Char('j')));
    }

    #[test]
    fn test_parse_key_string_up_down() {
        assert_eq!(parse_key_string("Up"), Some(BareKey::Up));
        assert_eq!(parse_key_string("Down"), Some(BareKey::Down));
    }

    #[test]
    fn test_parse_key_string_unknown_returns_none() {
        assert_eq!(parse_key_string("Ctrl+j"), None);
        assert_eq!(parse_key_string(""), None);
        assert_eq!(parse_key_string("unknown"), None);
    }

    fn make_state_with_tab(name: &str, pos: usize) -> PluginState {
        let mut s = PluginState::default();
        s.tab_entries = vec![crate::state::TabEntry {
            position: pos,
            name: name.into(),
            active: false,
            panes: vec![],
        }];
        s
    }

    #[test]
    fn test_enter_on_submenu_item_opens_submenu() {
        let mut state = PluginState::default();
        let groups = vec!["Backend".to_string()];
        let items = crate::menus::build_tab_menu(0, &groups);
        let submenu_idx = items
            .iter()
            .position(|i| matches!(&i.action, crate::menus::MenuAction::Submenu(l, _) if l == "Move to Group"))
            .expect("should have Move to Group submenu");
        state.active_menu = Some(crate::state::MenuState {
            target: MenuTarget::Tab(0),
            selected_index: submenu_idx,
            position_line: 0,
            items_cache: Some(items.clone()),
            ..Default::default()
        });
        handle_key(&mut state, make_key(BareKey::Enter));
        let menu = state
            .active_menu
            .as_ref()
            .expect("menu should still be open");
        assert!(
            menu.parent_items.is_some(),
            "parent_items should be set after entering submenu"
        );
        let cache = menu
            .items_cache
            .as_ref()
            .expect("items_cache should be set");
        assert!(
            cache.iter().any(|i| matches!(&i.action, crate::menus::MenuAction::MoveToGroup(0, n) if n == "Backend")),
            "items_cache should contain MoveToGroup items"
        );
        assert_eq!(menu.selected_index, 0, "selected_index should reset to 0");
    }

    #[test]
    fn test_esc_in_submenu_returns_to_parent_not_dismiss() {
        let mut state = PluginState::default();
        let parent_items = crate::menus::build_tab_menu(0, &["Ops".to_string()]);
        let sub_items = vec![crate::menus::MenuItem {
            label: "→ Ops".into(),
            action: crate::menus::MenuAction::MoveToGroup(0, "Ops".into()),
            is_separator: false,
        }];
        state.active_menu = Some(crate::state::MenuState {
            target: MenuTarget::Tab(0),
            selected_index: 0,
            position_line: 0,
            parent_items: Some(parent_items.clone()),
            items_cache: Some(sub_items),
        });
        handle_key(&mut state, make_key(BareKey::Esc));
        let menu = state
            .active_menu
            .as_ref()
            .expect("menu should still be open after Esc in submenu");
        assert!(
            menu.parent_items.is_none(),
            "parent_items should be cleared after returning to parent"
        );
        let cache = menu
            .items_cache
            .as_ref()
            .expect("items_cache should be restored");
        assert_eq!(
            cache.len(),
            parent_items.len(),
            "items_cache should be restored to parent items"
        );
    }

    #[test]
    fn test_esc_in_root_menu_dismisses() {
        let mut state = PluginState::default();
        state.active_menu = Some(crate::state::MenuState {
            target: MenuTarget::Tab(0),
            selected_index: 0,
            position_line: 0,
            ..Default::default()
        });
        handle_key(&mut state, make_key(BareKey::Esc));
        assert!(
            state.active_menu.is_none(),
            "Esc in root menu should dismiss"
        );
    }

    #[test]
    fn test_submenu_item_select_executes_action_and_closes_menu() {
        let mut state = make_state_with_tab("api", 0);
        let sub_items = vec![crate::menus::MenuItem {
            label: "→ Backend".into(),
            action: crate::menus::MenuAction::MoveToGroup(0, "Backend".into()),
            is_separator: false,
        }];
        let parent_items = crate::menus::build_tab_menu(0, &["Backend".to_string()]);
        state.active_menu = Some(crate::state::MenuState {
            target: MenuTarget::Tab(0),
            selected_index: 0,
            position_line: 0,
            parent_items: Some(parent_items),
            items_cache: Some(sub_items),
        });
        handle_key(&mut state, make_key(BareKey::Enter));
        assert!(
            state.active_menu.is_none(),
            "menu should be dismissed after executing action"
        );
        assert_eq!(
            state.group_assignments.get(&TabKey::new("api", 0)),
            Some(&"Backend".into()),
            "MoveToGroup should have been executed"
        );
    }

    #[test]
    fn test_right_click_initialises_items_cache() {
        let mut state = PluginState::default();
        state.click_regions = vec![region(0, ClickTarget::Tab(3))];
        sim_right(&mut state, 0);
        let menu = state.active_menu.as_ref().expect("menu should be set");
        assert!(
            menu.items_cache.is_some(),
            "right-click should initialise items_cache"
        );
    }

    #[test]
    fn test_submenu_max_depth_one_level_ignored() {
        let mut state = PluginState::default();
        let nested = vec![crate::menus::MenuItem {
            label: "Inner".into(),
            action: crate::menus::MenuAction::Noop,
            is_separator: false,
        }];
        let sub_items = vec![crate::menus::MenuItem {
            label: "Nested ▶".into(),
            action: crate::menus::MenuAction::Submenu("Nested".into(), nested),
            is_separator: false,
        }];
        let parent_items = crate::menus::build_tab_menu(0, &[]);
        state.active_menu = Some(crate::state::MenuState {
            target: MenuTarget::Tab(0),
            selected_index: 0,
            position_line: 0,
            parent_items: Some(parent_items),
            items_cache: Some(sub_items),
        });
        handle_key(&mut state, make_key(BareKey::Enter));
        let menu = state
            .active_menu
            .as_ref()
            .expect("menu should still be open");
        assert!(
            menu.parent_items.is_some(),
            "parent_items should be unchanged — no deeper than 1 level"
        );
        assert_eq!(
            menu.selected_index, 0,
            "selected_index unchanged when depth guard fires"
        );
    }

    #[test]
    fn test_set_color_submenu_accessible_from_tab_menu() {
        let mut state = make_state_with_tab("api", 0);
        let groups: Vec<String> = vec![];
        let items = crate::menus::build_tab_menu(0, &groups);
        let color_submenu_idx = items
            .iter()
            .position(|i| matches!(&i.action, crate::menus::MenuAction::Submenu(l, _) if l == "Set Color"))
            .expect("should have Set Color submenu");
        state.active_menu = Some(crate::state::MenuState {
            target: MenuTarget::Tab(0),
            selected_index: color_submenu_idx,
            position_line: 0,
            items_cache: Some(items),
            ..Default::default()
        });
        handle_key(&mut state, make_key(BareKey::Enter));
        let menu = state
            .active_menu
            .as_ref()
            .expect("menu should still be open");
        let cache = menu
            .items_cache
            .as_ref()
            .expect("items_cache should have color items");
        assert!(
            cache.iter().any(
                |i| matches!(&i.action, crate::menus::MenuAction::SetColor(0, c) if c == "#e74c3c")
            ),
            "Set Color submenu should contain Red color option"
        );
    }

    #[test]
    fn test_picker_typing_filters_emoji() {
        let mut state = PluginState::default();
        state.active_picker = Some(crate::picker::EmojiPickerState::new(0));
        handle_key(&mut state, make_key(BareKey::Char('c')));
        handle_key(&mut state, make_key(BareKey::Char('a')));
        handle_key(&mut state, make_key(BareKey::Char('t')));
        let picker = state.active_picker.as_ref().unwrap();
        assert_eq!(picker.query, "cat");
        assert!(!picker.results.is_empty());
        for emoji in &picker.results {
            assert!(emoji.name().to_lowercase().contains("cat"));
        }
    }

    #[test]
    fn test_picker_backspace_removes_char() {
        let mut state = PluginState::default();
        state.active_picker = Some(crate::picker::EmojiPickerState::new(0));
        handle_key(&mut state, make_key(BareKey::Char('c')));
        handle_key(&mut state, make_key(BareKey::Char('a')));
        handle_key(&mut state, make_key(BareKey::Char('t')));
        handle_key(&mut state, make_key(BareKey::Backspace));
        let picker = state.active_picker.as_ref().unwrap();
        assert_eq!(picker.query, "ca");
    }

    #[test]
    fn test_picker_enter_sets_marker() {
        let mut state = make_state_with_tab("api", 0);
        let mut picker = crate::picker::EmojiPickerState::new(0);
        picker.query = "cat".into();
        picker.filter();
        state.active_picker = Some(picker);
        handle_key(&mut state, make_key(BareKey::Enter));
        assert!(
            state.active_picker.is_none(),
            "picker should be closed after Enter"
        );
        let key = TabKey::new("api", 0);
        assert!(state.markers.contains_key(&key), "marker should be set");
    }

    #[test]
    fn test_picker_esc_clears_picker() {
        let mut state = PluginState::default();
        state.active_picker = Some(crate::picker::EmojiPickerState::new(0));
        handle_key(&mut state, make_key(BareKey::Esc));
        assert!(state.active_picker.is_none());
    }

    #[test]
    fn test_picker_down_increments_selected() {
        let mut state = PluginState::default();
        state.active_picker = Some(crate::picker::EmojiPickerState::new(0));
        handle_key(&mut state, make_key(BareKey::Down));
        assert_eq!(state.active_picker.as_ref().unwrap().selected, 1);
    }

    #[test]
    fn test_picker_up_does_not_go_below_zero() {
        let mut state = PluginState::default();
        state.active_picker = Some(crate::picker::EmojiPickerState::new(0));
        handle_key(&mut state, make_key(BareKey::Up));
        assert_eq!(state.active_picker.as_ref().unwrap().selected, 0);
    }

    #[test]
    fn test_picker_active_intercepts_keys_before_cursor() {
        let mut state = PluginState::default();
        state.active_picker = Some(crate::picker::EmojiPickerState::new(0));
        state.click_regions = vec![
            ClickRegion {
                line: 0,
                target: ClickTarget::Tab(0),
            },
            ClickRegion {
                line: 1,
                target: ClickTarget::Tab(1),
            },
        ];
        state.cursor_position = Some(0);
        handle_key(&mut state, make_key(BareKey::Down));
        assert_eq!(
            state.cursor_position,
            Some(0),
            "cursor should not move while picker is active"
        );
    }

    #[test]
    fn test_color_picker_down_moves_to_next_component() {
        let mut state = PluginState::default();
        state.active_color_picker = Some(crate::color_picker::ColorPickerState::new(0, None));
        handle_key(&mut state, make_key(BareKey::Down));
        assert!(matches!(
            state.active_color_picker.as_ref().map(|cp| &cp.active),
            Some(crate::color_picker::HslComponent::S)
        ));
    }

    #[test]
    fn test_color_picker_enter_applies_color_and_clears_picker() {
        let mut state = make_state_with_tab("api", 0);
        let cp = crate::color_picker::ColorPickerState::new(0, Some("#ff0000"));
        let expected = cp.current_hex();
        state.active_color_picker = Some(cp);
        handle_key(&mut state, make_key(BareKey::Enter));
        assert!(state.active_color_picker.is_none());
        assert_eq!(
            state.custom_colors.get(&TabKey::new("api", 0)),
            Some(&expected)
        );
    }

    #[test]
    fn test_color_picker_esc_clears_without_applying() {
        let mut state = make_state_with_tab("api", 0);
        state.active_color_picker = Some(crate::color_picker::ColorPickerState::new(0, None));
        handle_key(&mut state, make_key(BareKey::Esc));
        assert!(state.active_color_picker.is_none());
        assert!(state.custom_colors.get(&TabKey::new("api", 0)).is_none());
    }
}
