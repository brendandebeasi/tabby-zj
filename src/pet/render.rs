use super::state::PetState;

pub fn render_pet(pet: &PetState, frame: &str, cols: usize) -> Vec<String> {
    let mut lines = Vec::new();

    let name_line = format!("{} ", pet.name);
    let name_visible = name_line.chars().count();
    if name_visible >= cols {
        lines.push(name_line.chars().take(cols).collect());
    } else {
        lines.push(format!("{}{}", name_line, " ".repeat(cols - name_visible)));
    }

    let sprite_visible = frame.chars().count();
    if sprite_visible >= cols {
        lines.push(frame.chars().take(cols).collect());
    } else {
        lines.push(format!("{}{}", frame, " ".repeat(cols - sprite_visible)));
    }

    let status = render_status_bar(pet, cols);
    lines.push(status);

    lines
}

fn render_status_bar(pet: &PetState, cols: usize) -> String {
    let hunger_bar = mini_bar(pet.hunger);
    let happy_bar = mini_bar(pet.happiness);
    let energy_bar = mini_bar(pet.energy);
    let status = format!("H{} J{} E{}", hunger_bar, happy_bar, energy_bar);
    let visible = status.chars().count();
    if visible >= cols {
        status.chars().take(cols).collect()
    } else {
        format!("{}{}", status, " ".repeat(cols - visible))
    }
}

fn mini_bar(value: f64) -> String {
    let filled = (value / 20.0).round() as usize;
    let empty = 5usize.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_pet_returns_3_lines() {
        let pet = PetState::default();
        let lines = render_pet(&pet, "(=^·ω·^=)", 25);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_render_pet_lines_padded_to_cols() {
        let pet = PetState::default();
        let lines = render_pet(&pet, "(=^·ω·^=)", 30);
        for line in &lines {
            assert_eq!(line.chars().count(), 30, "line not padded: {:?}", line);
        }
    }

    #[test]
    fn test_render_pet_name_appears() {
        let pet = PetState::new("Luna", 0);
        let lines = render_pet(&pet, "(=^·ω·^=)", 25);
        assert!(lines[0].contains("Luna"));
    }

    #[test]
    fn test_render_pet_sprite_appears() {
        let pet = PetState::default();
        let frame = "(=^◕ᴥ◕^=)";
        let lines = render_pet(&pet, frame, 25);
        assert!(lines[1].contains(frame));
    }

    #[test]
    fn test_render_status_bar_contains_bars() {
        let pet = PetState::default();
        let lines = render_pet(&pet, "x", 40);
        assert!(lines[2].contains("H["), "status should have hunger bar");
        assert!(lines[2].contains("J["), "status should have happiness bar");
        assert!(lines[2].contains("E["), "status should have energy bar");
    }

    #[test]
    fn test_mini_bar_full() {
        assert_eq!(mini_bar(100.0), "[█████]");
    }

    #[test]
    fn test_mini_bar_empty() {
        assert_eq!(mini_bar(0.0), "[░░░░░]");
    }

    #[test]
    fn test_mini_bar_half() {
        assert_eq!(mini_bar(50.0), "[███░░]");
    }

    #[test]
    fn test_render_narrow_cols_truncates() {
        let pet = PetState::default();
        let lines = render_pet(&pet, "(=^·ω·^=)", 5);
        for line in &lines {
            assert!(line.chars().count() <= 5 || line.chars().count() == 5);
        }
    }
}
