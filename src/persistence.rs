use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

#[derive(Default, Serialize, Deserialize)]
pub struct PersistedState {
    pub group_assignments: HashMap<String, String>,
    pub collapsed_groups: HashSet<String>,
    pub custom_colors: HashMap<String, String>,
    pub markers: HashMap<String, String>,
    pub sidebar_collapsed: bool,
}

fn state_path() -> PathBuf {
    let dir = std::env::var("TABBY_ZJ_STATE_DIR").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        format!("{}/.local/state/tabby-zj", home)
    });
    PathBuf::from(dir).join("state.json")
}

pub fn save_state(state: &PersistedState) {
    let path = state_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(&path, json);
    }
}

pub fn load_state() -> PersistedState {
    let path = state_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_valid_json() {
        let json = r##"{
            "group_assignments": {"api::0": "Backend"},
            "collapsed_groups": ["Frontend"],
            "custom_colors": {"web::1": "#e74c3c"},
            "markers": {"api::0": "🚀"},
            "sidebar_collapsed": true
        }"##;
        let state: PersistedState = serde_json::from_str(json).unwrap();
        assert_eq!(
            state.group_assignments.get("api::0"),
            Some(&"Backend".into())
        );
        assert!(state.collapsed_groups.contains("Frontend"));
        assert_eq!(state.custom_colors.get("web::1"), Some(&"#e74c3c".into()));
        assert_eq!(state.markers.get("api::0"), Some(&"🚀".into()));
        assert!(state.sidebar_collapsed);
    }

    #[test]
    fn test_corrupt_json_deserialize_falls_back() {
        let result = serde_json::from_str::<PersistedState>("not valid json{{");
        assert!(result.is_err());
        let s = result.unwrap_or_default();
        assert!(s.group_assignments.is_empty());
        assert!(!s.sidebar_collapsed);
    }

    #[test]
    fn test_serialize_roundtrip() {
        let mut state = PersistedState::default();
        state
            .group_assignments
            .insert("api::0".into(), "Backend".into());
        state.collapsed_groups.insert("Frontend".into());
        state
            .custom_colors
            .insert("web::1".into(), "#e74c3c".into());
        state.markers.insert("api::0".into(), "🚀".into());
        state.sidebar_collapsed = true;

        let json = serde_json::to_string_pretty(&state).unwrap();
        let loaded: PersistedState = serde_json::from_str(&json).unwrap();

        assert_eq!(
            loaded.group_assignments.get("api::0"),
            Some(&"Backend".into())
        );
        assert!(loaded.collapsed_groups.contains("Frontend"));
        assert_eq!(loaded.custom_colors.get("web::1"), Some(&"#e74c3c".into()));
        assert_eq!(loaded.markers.get("api::0"), Some(&"🚀".into()));
        assert!(loaded.sidebar_collapsed);
    }

    #[test]
    fn test_save_load_disk_roundtrip() {
        let dir = std::env::temp_dir().join(format!("tabby-zj-persist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("TABBY_ZJ_STATE_DIR", dir.to_str().unwrap());

        let mut state = PersistedState::default();
        state
            .group_assignments
            .insert("api::0".into(), "Backend".into());
        state.sidebar_collapsed = true;
        save_state(&state);

        let loaded = load_state();
        std::env::remove_var("TABBY_ZJ_STATE_DIR");
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(
            loaded.group_assignments.get("api::0"),
            Some(&"Backend".into())
        );
        assert!(loaded.sidebar_collapsed);
    }
}
