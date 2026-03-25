#[derive(Clone, Debug, PartialEq)]
pub enum ClickTarget {
    Tab(usize),
    Pane(u32),
    Group(String),
    #[allow(dead_code)]
    Empty,
}

#[derive(Clone, Debug)]
pub struct ClickRegion {
    pub line: usize,
    pub target: ClickTarget,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_click_target_tab_variant() {
        let t = ClickTarget::Tab(3);
        assert!(matches!(t, ClickTarget::Tab(3)));
    }

    #[test]
    fn test_click_target_group_variant() {
        let t = ClickTarget::Group("Frontend".into());
        assert!(matches!(t, ClickTarget::Group(ref s) if s == "Frontend"));
    }

    #[test]
    fn test_click_target_empty_variant() {
        let t = ClickTarget::Empty;
        assert!(matches!(t, ClickTarget::Empty));
    }

    #[test]
    fn test_click_region_line_and_target() {
        let r = ClickRegion {
            line: 5,
            target: ClickTarget::Tab(2),
        };
        assert_eq!(r.line, 5);
        assert!(matches!(r.target, ClickTarget::Tab(2)));
    }

    #[test]
    fn test_click_target_pane_variant() {
        let t = ClickTarget::Pane(42);
        assert!(matches!(t, ClickTarget::Pane(42)));
    }
}
