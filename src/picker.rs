use unicode_width::UnicodeWidthStr;

#[derive(Clone)]
pub struct EmojiPickerState {
    pub query: String,
    pub results: Vec<&'static emojis::Emoji>,
    pub selected: usize,
    pub target_tab: usize,
}

impl EmojiPickerState {
    pub fn new(target_tab: usize) -> Self {
        let results = emojis::iter().collect();
        Self {
            query: String::new(),
            results,
            selected: 0,
            target_tab,
        }
    }

    pub fn filter(&mut self) {
        let q = self.query.to_lowercase();
        self.results = emojis::iter()
            .filter(|e| q.is_empty() || e.name().to_lowercase().contains(&q))
            .collect();
        self.selected = 0;
    }

    pub fn selected_emoji(&self) -> Option<&str> {
        self.results.get(self.selected).map(|e| e.as_str())
    }
}

pub fn render_picker(picker: &EmojiPickerState, cols: usize) -> Vec<String> {
    let cols = cols.max(4);
    let mut lines = Vec::new();

    let header_text = format!("Search: {}\u{2588}", picker.query);
    let header_vis = UnicodeWidthStr::width(header_text.as_str());
    let header_pad = if header_vis < cols {
        " ".repeat(cols - header_vis)
    } else {
        String::new()
    };
    lines.push(format!("\x1b[7m{}{}\x1b[0m", header_text, header_pad));

    if picker.results.is_empty() {
        let msg = "  No results";
        let w = UnicodeWidthStr::width(msg);
        let pad = if w < cols {
            " ".repeat(cols - w)
        } else {
            String::new()
        };
        lines.push(format!("{}{}", msg, pad));
        return lines;
    }

    let slot_cols: usize = 3;
    let emojis_per_row = (cols / slot_cols).max(1);

    for (row_idx, chunk) in picker.results.chunks(emojis_per_row).enumerate() {
        let mut row_str = String::new();
        let mut vis_w: usize = 0;

        for (col_idx, emoji) in chunk.iter().enumerate() {
            let global_idx = row_idx * emojis_per_row + col_idx;
            let ch = emoji.as_str();
            let ew = UnicodeWidthStr::width(ch);
            if global_idx == picker.selected {
                row_str.push_str("\x1b[7m");
                row_str.push_str(ch);
                row_str.push_str("\x1b[0m");
            } else {
                row_str.push_str(ch);
            }
            row_str.push(' ');
            vis_w += ew + 1;
        }

        if vis_w < cols {
            row_str.push_str(&" ".repeat(cols - vis_w));
        }

        lines.push(row_str);
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_has_non_empty_results() {
        let picker = EmojiPickerState::new(0);
        assert!(!picker.results.is_empty());
    }

    #[test]
    fn test_new_query_is_empty() {
        let picker = EmojiPickerState::new(0);
        assert_eq!(picker.query, "");
    }

    #[test]
    fn test_new_selected_is_zero() {
        let picker = EmojiPickerState::new(0);
        assert_eq!(picker.selected, 0);
    }

    #[test]
    fn test_new_target_tab() {
        let picker = EmojiPickerState::new(7);
        assert_eq!(picker.target_tab, 7);
    }

    #[test]
    fn test_empty_query_returns_all() {
        let picker = EmojiPickerState::new(0);
        let all_count = emojis::iter().count();
        assert_eq!(picker.results.len(), all_count);
    }

    #[test]
    fn test_filter_cat_returns_cat_emoji() {
        let mut picker = EmojiPickerState::new(0);
        picker.query = "cat".into();
        picker.filter();
        assert!(
            !picker.results.is_empty(),
            "filtering 'cat' should return results"
        );
        for emoji in &picker.results {
            assert!(
                emoji.name().to_lowercase().contains("cat"),
                "expected 'cat' in name '{}' but wasn't",
                emoji.name()
            );
        }
    }

    #[test]
    fn test_filter_resets_selected_to_zero() {
        let mut picker = EmojiPickerState::new(0);
        picker.selected = 5;
        picker.query = "cat".into();
        picker.filter();
        assert_eq!(picker.selected, 0);
    }

    #[test]
    fn test_filter_empty_query_shows_all() {
        let mut picker = EmojiPickerState::new(0);
        picker.query = "cat".into();
        picker.filter();
        let cat_count = picker.results.len();
        picker.query.clear();
        picker.filter();
        let all_count = emojis::iter().count();
        assert!(picker.results.len() > cat_count);
        assert_eq!(picker.results.len(), all_count);
    }

    #[test]
    fn test_selected_emoji_returns_correct() {
        let mut picker = EmojiPickerState::new(0);
        picker.query = "cat".into();
        picker.filter();
        let emoji = picker.selected_emoji();
        assert!(emoji.is_some());
    }

    #[test]
    fn test_selected_emoji_safe_when_empty_results() {
        let mut picker = EmojiPickerState::new(0);
        picker.query = "xyznotanemoji999abc".into();
        picker.filter();
        let _ = picker.selected_emoji();
    }

    #[test]
    fn test_backspace_removes_last_char() {
        let mut picker = EmojiPickerState::new(0);
        picker.query = "cat".into();
        picker.query.pop();
        picker.filter();
        assert_eq!(picker.query, "ca");
    }

    #[test]
    fn test_clone_works() {
        let picker = EmojiPickerState::new(3);
        let cloned = picker.clone();
        assert_eq!(cloned.target_tab, 3);
        assert_eq!(cloned.results.len(), picker.results.len());
    }

    #[test]
    fn test_render_picker_has_header_as_first_line() {
        let picker = EmojiPickerState::new(0);
        let lines = render_picker(&picker, 20);
        assert!(!lines.is_empty());
        assert!(lines[0].contains("Search:"));
        assert!(lines[0].contains('\u{2588}'));
    }

    #[test]
    fn test_render_picker_header_contains_query() {
        let mut picker = EmojiPickerState::new(0);
        picker.query = "cat".into();
        picker.filter();
        let lines = render_picker(&picker, 30);
        assert!(lines[0].contains("cat"));
    }

    #[test]
    fn test_render_picker_no_results_shows_message() {
        let mut picker = EmojiPickerState::new(0);
        picker.query = "xyznotanemoji999abc".into();
        picker.filter();
        if picker.results.is_empty() {
            let lines = render_picker(&picker, 20);
            assert!(lines.iter().any(|l| l.contains("No results")));
        }
    }

    #[test]
    fn test_render_picker_multiple_lines_when_results_exist() {
        let picker = EmojiPickerState::new(0);
        let lines = render_picker(&picker, 20);
        assert!(lines.len() >= 2);
    }

    #[test]
    fn test_render_picker_selected_highlighted_with_reverse_video() {
        let mut picker = EmojiPickerState::new(0);
        picker.query = "cat".into();
        picker.filter();
        if !picker.results.is_empty() {
            picker.selected = 0;
            let lines = render_picker(&picker, 30);
            let all = lines.join("");
            assert!(all.contains("\x1b[7m"));
        }
    }
}
