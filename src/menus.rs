use crate::persistence;
use crate::state::{MenuTarget, PluginState, RenameState, RenameTarget, TabKey};
#[allow(unused_imports)]
use zellij_tile::prelude::*;

#[derive(Clone, Debug)]
pub struct MenuItem {
    pub label: String,
    pub action: MenuAction,
    pub is_separator: bool,
}

#[derive(Clone, Debug)]
pub enum MenuAction {
    Noop,
    SwitchTab(usize),
    FocusPane(u32),
    CloseTab(usize),
    ClosePane(u32),
    RenameTab(usize),
    RenamePane(u32),
    RenameGroup(String),
    ToggleGroup(String),
    MoveToGroup(usize, String),
    NewTabInGroup(String),
    Submenu(String, Vec<MenuItem>),
    SetColor(usize, String),
}

fn sep() -> MenuItem {
    MenuItem {
        label: String::new(),
        action: MenuAction::Noop,
        is_separator: true,
    }
}

fn item(label: &str, action: MenuAction) -> MenuItem {
    MenuItem {
        label: label.into(),
        action,
        is_separator: false,
    }
}

pub fn build_tab_menu(tab_index: usize, group_names: &[String]) -> Vec<MenuItem> {
    let mut items = vec![
        item("▶ Switch to tab", MenuAction::SwitchTab(tab_index)),
        sep(),
        item("✎ Rename", MenuAction::RenameTab(tab_index)),
    ];
    if !group_names.is_empty() {
        let sub_items: Vec<MenuItem> = group_names
            .iter()
            .map(|n| {
                item(
                    &format!("→ {}", n),
                    MenuAction::MoveToGroup(tab_index, n.clone()),
                )
            })
            .collect();
        items.push(sep());
        items.push(item(
            "→ Move to Group ▶",
            MenuAction::Submenu("Move to Group".into(), sub_items),
        ));
    }
    let color_items = vec![
        item("● Red", MenuAction::SetColor(tab_index, "#e74c3c".into())),
        item(
            "● Orange",
            MenuAction::SetColor(tab_index, "#e67e22".into()),
        ),
        item(
            "● Yellow",
            MenuAction::SetColor(tab_index, "#f1c40f".into()),
        ),
        item("● Green", MenuAction::SetColor(tab_index, "#27ae60".into())),
        item("● Blue", MenuAction::SetColor(tab_index, "#3498db".into())),
        item(
            "● Purple",
            MenuAction::SetColor(tab_index, "#9b59b6".into()),
        ),
        item("● Pink", MenuAction::SetColor(tab_index, "#e91e63".into())),
        item("● Cyan", MenuAction::SetColor(tab_index, "#1abc9c".into())),
        item("● Gray", MenuAction::SetColor(tab_index, "#95a5a6".into())),
    ];
    items.push(sep());
    items.push(item(
        "◉ Set Color ▶",
        MenuAction::Submenu("Set Color".into(), color_items),
    ));
    items.push(sep());
    items.push(item("✕ Close tab", MenuAction::CloseTab(tab_index)));
    items
}

pub fn build_pane_menu(pane_id: u32) -> Vec<MenuItem> {
    vec![
        item("▶ Focus pane", MenuAction::FocusPane(pane_id)),
        sep(),
        item("✎ Rename pane", MenuAction::RenamePane(pane_id)),
        item("✕ Close pane", MenuAction::ClosePane(pane_id)),
    ]
}

#[allow(dead_code)]
pub fn build_menu_for_target(target: &MenuTarget, group_names: &[String]) -> Vec<MenuItem> {
    match target {
        MenuTarget::Tab(pos) => build_tab_menu(*pos, group_names),
        MenuTarget::Pane(id) => build_pane_menu(*id),
        MenuTarget::Group(name) => build_group_menu(name),
        MenuTarget::None => vec![],
    }
}

pub fn build_group_menu(group_name: &str) -> Vec<MenuItem> {
    vec![
        item(
            "⊟ Collapse/Expand",
            MenuAction::ToggleGroup(group_name.into()),
        ),
        sep(),
        item("✎ Rename group", MenuAction::RenameGroup(group_name.into())),
        item(
            "+ New tab in group",
            MenuAction::NewTabInGroup(group_name.into()),
        ),
    ]
}

pub fn execute_action(state: &mut PluginState, action: MenuAction) {
    state.active_menu = None;
    match action {
        MenuAction::Noop => {}
        MenuAction::SwitchTab(pos) => {
            #[cfg(not(test))]
            switch_tab_to(pos as u32);
            let _ = pos;
        }
        MenuAction::FocusPane(pane_id) => {
            #[cfg(not(test))]
            show_pane_with_id(PaneId::Terminal(pane_id), false, true);
            let _ = pane_id;
        }
        MenuAction::CloseTab(idx) => {
            #[cfg(not(test))]
            close_tab_with_index(idx);
            let _ = idx;
        }
        MenuAction::ClosePane(pane_id) => {
            #[cfg(not(test))]
            close_multiple_panes(vec![PaneId::Terminal(pane_id)]);
            let _ = pane_id;
        }
        MenuAction::RenameTab(pos) => {
            state.rename_state = Some(RenameState {
                target: RenameTarget::Tab(pos),
                buffer: String::new(),
            });
        }
        MenuAction::RenamePane(pane_id) => {
            state.rename_state = Some(RenameState {
                target: RenameTarget::Pane(pane_id),
                buffer: String::new(),
            });
        }
        MenuAction::RenameGroup(name) => {
            state.rename_state = Some(RenameState {
                target: RenameTarget::Group(name.clone()),
                buffer: name,
            });
        }
        MenuAction::ToggleGroup(name) => {
            if state.collapsed_groups.contains(&name) {
                state.collapsed_groups.remove(&name);
            } else {
                state.collapsed_groups.insert(name);
            }
            flush_state(state);
        }
        MenuAction::MoveToGroup(tab_pos, group_name) => {
            if let Some(key) = state
                .tab_entries
                .iter()
                .find(|t| t.position == tab_pos)
                .map(|t| TabKey::new(&t.name, t.position))
            {
                state.group_assignments.insert(key, group_name);
                flush_state(state);
            }
        }
        MenuAction::NewTabInGroup(_group_name) => {
            #[cfg(not(test))]
            {
                let _ = new_tab::<String>(None, None);
            }
        }
        MenuAction::SetColor(tab_pos, color) => {
            if let Some(key) = state
                .tab_entries
                .iter()
                .find(|t| t.position == tab_pos)
                .map(|t| TabKey::new(&t.name, t.position))
            {
                state.custom_colors.insert(key, color);
                flush_state(state);
            }
        }
        MenuAction::Submenu(_, _) => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{PluginState, TabEntry};

    fn make_state() -> PluginState {
        PluginState::default()
    }

    fn make_state_with_tab(name: &str, pos: usize) -> PluginState {
        let mut s = make_state();
        s.tab_entries = vec![TabEntry {
            position: pos,
            name: name.into(),
            active: false,
            panes: vec![],
        }];
        s
    }

    #[test]
    fn test_build_tab_menu_has_switch_and_close() {
        let items = build_tab_menu(2, &[]);
        assert!(items
            .iter()
            .any(|i| matches!(i.action, MenuAction::SwitchTab(2))));
        assert!(items
            .iter()
            .any(|i| matches!(i.action, MenuAction::CloseTab(2))));
    }

    #[test]
    fn test_build_pane_menu_has_focus_and_close() {
        let items = build_pane_menu(42);
        assert!(items
            .iter()
            .any(|i| matches!(i.action, MenuAction::FocusPane(42))));
        assert!(items
            .iter()
            .any(|i| matches!(i.action, MenuAction::ClosePane(42))));
    }

    #[test]
    fn test_build_group_menu_has_toggle_and_rename() {
        let items = build_group_menu("Backend");
        assert!(items
            .iter()
            .any(|i| matches!(&i.action, MenuAction::ToggleGroup(n) if n == "Backend")));
        assert!(items
            .iter()
            .any(|i| matches!(&i.action, MenuAction::RenameGroup(n) if n == "Backend")));
    }

    #[test]
    fn test_execute_toggle_group_collapses() {
        let mut state = make_state();
        execute_action(&mut state, MenuAction::ToggleGroup("Frontend".into()));
        assert!(state.collapsed_groups.contains("Frontend"));
        assert!(state.active_menu.is_none());
    }

    #[test]
    fn test_execute_toggle_group_expands() {
        let mut state = make_state();
        state.collapsed_groups.insert("Frontend".into());
        execute_action(&mut state, MenuAction::ToggleGroup("Frontend".into()));
        assert!(!state.collapsed_groups.contains("Frontend"));
    }

    #[test]
    fn test_execute_move_to_group_assigns() {
        let mut state = make_state_with_tab("api", 0);
        execute_action(&mut state, MenuAction::MoveToGroup(0, "Backend".into()));
        assert_eq!(
            state.group_assignments.get(&TabKey::new("api", 0)),
            Some(&"Backend".into())
        );
    }

    #[test]
    fn test_execute_rename_tab_sets_state() {
        let mut state = make_state();
        execute_action(&mut state, MenuAction::RenameTab(1));
        assert!(matches!(
            state.rename_state.as_ref().map(|r| &r.target),
            Some(RenameTarget::Tab(1))
        ));
    }

    #[test]
    fn test_execute_rename_group_prefills_buffer() {
        let mut state = make_state();
        execute_action(&mut state, MenuAction::RenameGroup("Frontend".into()));
        let rs = state.rename_state.as_ref().unwrap();
        assert!(matches!(&rs.target, RenameTarget::Group(n) if n == "Frontend"));
        assert_eq!(rs.buffer, "Frontend");
    }

    #[test]
    fn test_execute_clears_active_menu() {
        let mut state = make_state();
        state.active_menu = Some(crate::state::MenuState::default());
        execute_action(&mut state, MenuAction::Noop);
        assert!(state.active_menu.is_none());
    }

    #[test]
    fn test_separators_are_not_selectable() {
        let items = build_tab_menu(0, &[]);
        let seps: Vec<_> = items.iter().filter(|i| i.is_separator).collect();
        assert!(
            !seps.is_empty(),
            "tab menu should have at least one separator"
        );
        for sep in seps {
            assert!(sep.label.is_empty(), "separator should have empty label");
        }
    }

    #[test]
    fn test_build_tab_menu_no_groups_no_move_items() {
        let items = build_tab_menu(3, &[]);
        let move_items: Vec<_> = items
            .iter()
            .filter(|i| matches!(&i.action, MenuAction::MoveToGroup(_, _)))
            .collect();
        assert!(
            move_items.is_empty(),
            "no group names → no MoveToGroup items"
        );
    }

    #[test]
    fn test_build_tab_menu_with_groups_emits_move_items() {
        let groups = vec!["Frontend".to_string(), "Backend".to_string()];
        let items = build_tab_menu(1, &groups);
        let submenu = items
            .iter()
            .find(
                |i| matches!(&i.action, MenuAction::Submenu(label, _) if label == "Move to Group"),
            )
            .expect("should have Move to Group submenu");
        let sub_items = match &submenu.action {
            MenuAction::Submenu(_, sub_items) => sub_items,
            _ => panic!("expected submenu"),
        };
        assert_eq!(sub_items.len(), 2, "should have one MoveToGroup per group");
        assert!(sub_items
            .iter()
            .any(|i| matches!(&i.action, MenuAction::MoveToGroup(1, n) if n == "Frontend")));
        assert!(sub_items
            .iter()
            .any(|i| matches!(&i.action, MenuAction::MoveToGroup(1, n) if n == "Backend")));
    }

    #[test]
    fn test_build_tab_menu_move_items_have_arrow_label() {
        let groups = vec!["Infra".to_string()];
        let items = build_tab_menu(0, &groups);
        let submenu = items
            .iter()
            .find(
                |i| matches!(&i.action, MenuAction::Submenu(label, _) if label == "Move to Group"),
            )
            .expect("should have Move to Group submenu");
        let sub_items = match &submenu.action {
            MenuAction::Submenu(_, sub_items) => sub_items,
            _ => panic!("expected submenu"),
        };
        let move_item = sub_items
            .iter()
            .find(|i| matches!(&i.action, MenuAction::MoveToGroup(_, _)))
            .expect("submenu should have a MoveToGroup item");
        assert!(
            move_item.label.contains("Infra"),
            "MoveToGroup label should contain group name"
        );
        assert!(
            move_item.label.contains('→'),
            "MoveToGroup label should have arrow indicator"
        );
    }

    #[test]
    fn test_build_tab_menu_with_groups_still_has_close() {
        let groups = vec!["Frontend".to_string()];
        let items = build_tab_menu(2, &groups);
        assert!(
            items
                .iter()
                .any(|i| matches!(i.action, MenuAction::CloseTab(2))),
            "CloseTab should still be present when groups are provided"
        );
    }

    #[test]
    fn test_build_menu_for_target_tab_passes_groups() {
        let groups = vec!["Ops".to_string()];
        let items = build_menu_for_target(&MenuTarget::Tab(5), &groups);
        let submenu = items
            .iter()
            .find(
                |i| matches!(&i.action, MenuAction::Submenu(label, _) if label == "Move to Group"),
            )
            .expect("build_menu_for_target should produce Move to Group submenu");
        let sub_items = match &submenu.action {
            MenuAction::Submenu(_, sub_items) => sub_items,
            _ => panic!("expected submenu"),
        };
        assert!(
            sub_items
                .iter()
                .any(|i| matches!(&i.action, MenuAction::MoveToGroup(5, n) if n == "Ops")),
            "submenu should contain MoveToGroup item forwarded from group_names"
        );
    }

    #[test]
    fn test_build_menu_for_target_pane_ignores_groups() {
        let groups = vec!["Frontend".to_string()];
        let items = build_menu_for_target(&MenuTarget::Pane(99), &groups);
        assert!(
            items
                .iter()
                .any(|i| matches!(i.action, MenuAction::FocusPane(99))),
            "pane menu should still have FocusPane"
        );
        assert!(
            items
                .iter()
                .all(|i| !matches!(&i.action, MenuAction::MoveToGroup(_, _))),
            "pane menu should not have MoveToGroup items"
        );
    }

    #[test]
    fn test_build_tab_menu_always_has_set_color_submenu() {
        let items = build_tab_menu(0, &[]);
        let submenu = items
            .iter()
            .find(|i| matches!(&i.action, MenuAction::Submenu(label, _) if label == "Set Color"))
            .expect("tab menu should always have Set Color submenu");
        let sub_items = match &submenu.action {
            MenuAction::Submenu(_, sub_items) => sub_items,
            _ => panic!("expected submenu"),
        };
        assert_eq!(
            sub_items.len(),
            9,
            "Set Color submenu should have 9 color options"
        );
        assert!(sub_items
            .iter()
            .any(|i| matches!(&i.action, MenuAction::SetColor(0, c) if c == "#e74c3c")));
        assert!(sub_items.iter().any(|i| i.label.contains("Red")));
    }

    #[test]
    fn test_build_tab_menu_set_color_submenu_label_has_indicator() {
        let items = build_tab_menu(3, &[]);
        let submenu = items
            .iter()
            .find(|i| matches!(&i.action, MenuAction::Submenu(label, _) if label == "Set Color"))
            .expect("should have Set Color submenu");
        assert!(
            submenu.label.contains('▶'),
            "Set Color submenu label should have ▶"
        );
    }

    #[test]
    fn test_execute_set_color_inserts_custom_color() {
        let mut state = make_state_with_tab("api", 0);
        execute_action(&mut state, MenuAction::SetColor(0, "#e74c3c".into()));
        assert_eq!(
            state.custom_colors.get(&TabKey::new("api", 0)),
            Some(&"#e74c3c".into()),
            "SetColor should store the color in custom_colors"
        );
    }

    #[test]
    fn test_execute_set_color_clears_active_menu() {
        let mut state = make_state_with_tab("api", 0);
        state.active_menu = Some(crate::state::MenuState::default());
        execute_action(&mut state, MenuAction::SetColor(0, "#27ae60".into()));
        assert!(state.active_menu.is_none());
    }

    #[test]
    fn test_execute_submenu_is_noop() {
        let mut state = make_state();
        state.active_menu = Some(crate::state::MenuState::default());
        execute_action(&mut state, MenuAction::Submenu("Test".into(), vec![]));
        assert!(
            state.active_menu.is_none(),
            "execute_action should still clear menu on Submenu"
        );
    }
}
