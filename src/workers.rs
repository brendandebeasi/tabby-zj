use std::collections::BTreeMap;
use std::path::PathBuf;

pub const CTX_TYPE_GIT: &str = "git_status";
pub const GIT_POLL_INTERVAL: u64 = 30;

pub fn request_git_status(cwd: PathBuf) {
    let mut ctx = BTreeMap::new();
    ctx.insert("type".to_string(), CTX_TYPE_GIT.to_string());
    #[cfg(not(test))]
    {
        use zellij_tile::prelude::run_command_with_env_variables_and_cwd;
        run_command_with_env_variables_and_cwd(
            &["git", "status", "--porcelain=v1", "-b"],
            BTreeMap::new(),
            cwd,
            ctx,
        );
    }
    #[cfg(test)]
    let _ = (cwd, ctx);
}

pub fn parse_git_status(output: &str) -> crate::state::GitStatus {
    let mut status = crate::state::GitStatus::default();

    for (i, line) in output.lines().enumerate() {
        if i == 0 {
            if let Some(rest) = line.strip_prefix("## ") {
                let branch_end = rest.find("...").unwrap_or(rest.len());
                status.branch = rest[..branch_end].to_string();

                if let Some(bracket_start) = rest.find('[') {
                    let bracket_end = rest.rfind(']').unwrap_or(rest.len());
                    let inside = &rest[bracket_start + 1..bracket_end];
                    for part in inside.split(',') {
                        let part = part.trim();
                        if let Some(n) = part.strip_prefix("ahead ").and_then(|s| s.parse().ok()) {
                            status.ahead = n;
                        }
                        if let Some(n) = part.strip_prefix("behind ").and_then(|s| s.parse().ok()) {
                            status.behind = n;
                        }
                    }
                }
            }
            continue;
        }

        if line.len() < 2 {
            continue;
        }

        let xy: Vec<char> = line.chars().take(2).collect();
        let x = xy[0];
        let y = xy[1];

        if x == '?' && y == '?' {
            status.dirty += 1;
        } else {
            if x != ' ' {
                status.staged += 1;
            }
            if y != ' ' {
                status.dirty += 1;
            }
        }
    }

    status
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_output() {
        let s = parse_git_status("");
        assert_eq!(s.branch, "");
        assert_eq!(s.dirty, 0);
        assert_eq!(s.staged, 0);
        assert_eq!(s.ahead, 0);
        assert_eq!(s.behind, 0);
    }

    #[test]
    fn test_parse_clean_branch() {
        let out = "## main...origin/main\n";
        let s = parse_git_status(out);
        assert_eq!(s.branch, "main");
        assert_eq!(s.dirty, 0);
        assert_eq!(s.staged, 0);
    }

    #[test]
    fn test_parse_ahead_behind() {
        let out = "## feat/x...origin/feat/x [ahead 3, behind 1]\n";
        let s = parse_git_status(out);
        assert_eq!(s.branch, "feat/x");
        assert_eq!(s.ahead, 3);
        assert_eq!(s.behind, 1);
    }

    #[test]
    fn test_parse_ahead_only() {
        let out = "## main...origin/main [ahead 2]\n";
        let s = parse_git_status(out);
        assert_eq!(s.ahead, 2);
        assert_eq!(s.behind, 0);
    }

    #[test]
    fn test_parse_dirty_untracked() {
        let out = "## main...origin/main\n?? new_file.rs\n?? another.rs\n";
        let s = parse_git_status(out);
        assert_eq!(s.dirty, 2);
        assert_eq!(s.staged, 0);
    }

    #[test]
    fn test_parse_staged_files() {
        let out = "## main...origin/main\nM  src/foo.rs\nA  src/bar.rs\n";
        let s = parse_git_status(out);
        assert_eq!(s.staged, 2);
        assert_eq!(s.dirty, 0);
    }

    #[test]
    fn test_parse_modified_unstaged() {
        let out = "## main...origin/main\n M src/foo.rs\n M src/bar.rs\n";
        let s = parse_git_status(out);
        assert_eq!(s.dirty, 2);
        assert_eq!(s.staged, 0);
    }

    #[test]
    fn test_parse_mixed_status() {
        let out = "## main...origin/main [ahead 1]\nM  staged.rs\n M dirty.rs\n?? untracked.rs\n";
        let s = parse_git_status(out);
        assert_eq!(s.branch, "main");
        assert_eq!(s.staged, 1);
        assert_eq!(s.dirty, 2);
        assert_eq!(s.ahead, 1);
    }

    #[test]
    fn test_parse_no_remote_branch() {
        let out = "## detached HEAD\n";
        let s = parse_git_status(out);
        assert_eq!(s.branch, "detached HEAD");
        assert_eq!(s.ahead, 0);
        assert_eq!(s.behind, 0);
    }

    #[test]
    fn test_parse_modified_both_staged_and_dirty() {
        let out = "## main...origin/main\nMM src/lib.rs\n";
        let s = parse_git_status(out);
        assert_eq!(s.staged, 1);
        assert_eq!(s.dirty, 1);
    }
}
