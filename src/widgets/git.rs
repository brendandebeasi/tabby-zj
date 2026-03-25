use crate::state::GitStatus;

pub fn render_git(status: &Option<GitStatus>) -> Vec<String> {
    let s = match status {
        Some(s) => s,
        None => return vec![],
    };

    let branch_line = format!(" {}", s.branch);

    let mut parts: Vec<String> = Vec::new();
    if s.staged > 0 {
        parts.push(format!("+{}", s.staged));
    }
    if s.dirty > 0 {
        parts.push(format!("~{}", s.dirty));
    }
    if s.ahead > 0 {
        parts.push(format!("↑{}", s.ahead));
    }
    if s.behind > 0 {
        parts.push(format!("↓{}", s.behind));
    }

    if parts.is_empty() {
        vec![branch_line, " ✓ clean".to_string()]
    } else {
        vec![branch_line, format!(" {}", parts.join(" "))]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_status(
        branch: &str,
        dirty: usize,
        staged: usize,
        ahead: usize,
        behind: usize,
    ) -> GitStatus {
        GitStatus {
            branch: branch.to_string(),
            dirty,
            staged,
            ahead,
            behind,
        }
    }

    #[test]
    fn test_none_returns_empty() {
        assert!(render_git(&None).is_empty());
    }

    #[test]
    fn test_clean_repo_shows_checkmark() {
        let lines = render_git(&Some(make_status("main", 0, 0, 0, 0)));
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("main"));
        assert!(lines[1].contains("clean"));
    }

    #[test]
    fn test_dirty_shows_tilde_count() {
        let lines = render_git(&Some(make_status("main", 3, 0, 0, 0)));
        assert!(lines[1].contains("~3"), "got: {:?}", lines[1]);
    }

    #[test]
    fn test_staged_shows_plus_count() {
        let lines = render_git(&Some(make_status("feat/x", 0, 2, 0, 0)));
        assert!(lines[1].contains("+2"), "got: {:?}", lines[1]);
    }

    #[test]
    fn test_ahead_behind_shown() {
        let lines = render_git(&Some(make_status("main", 0, 0, 1, 2)));
        assert!(lines[1].contains("↑1"), "got: {:?}", lines[1]);
        assert!(lines[1].contains("↓2"), "got: {:?}", lines[1]);
    }

    #[test]
    fn test_all_fields_combined() {
        let lines = render_git(&Some(make_status("dev", 1, 2, 3, 4)));
        assert!(lines[1].contains("+2"));
        assert!(lines[1].contains("~1"));
        assert!(lines[1].contains("↑3"));
        assert!(lines[1].contains("↓4"));
    }

    #[test]
    fn test_branch_name_in_first_line() {
        let lines = render_git(&Some(make_status("feature/my-branch", 0, 0, 0, 0)));
        assert!(lines[0].contains("feature/my-branch"));
    }
}
