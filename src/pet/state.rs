use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Mood {
    Ecstatic,
    Happy,
    #[default]
    Content,
    Bored,
    Hungry,
    Sad,
    Sleeping,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PetState {
    pub name: String,
    pub hunger: f64,
    pub happiness: f64,
    pub energy: f64,
    pub mood: Mood,
    pub last_fed_tick: u64,
    pub last_petted_tick: u64,
    pub last_played_tick: u64,
    pub is_sleeping: bool,
    pub birth_tick: u64,
    pub tick: u64,
}

impl Default for PetState {
    fn default() -> Self {
        Self {
            name: "Whiskers".into(),
            hunger: 80.0,
            happiness: 80.0,
            energy: 100.0,
            mood: Mood::Content,
            last_fed_tick: 0,
            last_petted_tick: 0,
            last_played_tick: 0,
            is_sleeping: false,
            birth_tick: 0,
            tick: 0,
        }
    }
}

impl PetState {
    pub fn new(name: &str, birth_tick: u64) -> Self {
        Self {
            name: name.to_string(),
            birth_tick,
            ..Default::default()
        }
    }

    /// Advance one tick: decay hunger/happiness/energy, derive mood.
    pub fn tick(&mut self) {
        self.tick += 1;

        if self.is_sleeping {
            self.energy = (self.energy + 0.5).min(100.0);
            if self.energy >= 90.0 {
                self.is_sleeping = false;
            }
        } else {
            self.energy = (self.energy - 0.05).max(0.0);
        }

        self.hunger = (self.hunger - 0.1).max(0.0);
        self.happiness = (self.happiness - 0.03).max(0.0);

        if self.energy < 10.0 && !self.is_sleeping {
            self.is_sleeping = true;
        }

        self.mood = self.derive_mood();
    }

    pub fn feed(&mut self) {
        if self.is_sleeping {
            return;
        }
        self.hunger = (self.hunger + 30.0).min(100.0);
        self.happiness = (self.happiness + 5.0).min(100.0);
        self.last_fed_tick = self.tick;
    }

    pub fn pet(&mut self) {
        if self.is_sleeping {
            return;
        }
        self.happiness = (self.happiness + 15.0).min(100.0);
        self.last_petted_tick = self.tick;
    }

    pub fn play(&mut self) {
        if self.is_sleeping {
            return;
        }
        self.happiness = (self.happiness + 20.0).min(100.0);
        self.energy = (self.energy - 10.0).max(0.0);
        self.hunger = (self.hunger - 5.0).max(0.0);
        self.last_played_tick = self.tick;
    }

    fn derive_mood(&self) -> Mood {
        if self.is_sleeping {
            return Mood::Sleeping;
        }
        if self.hunger < 20.0 {
            return Mood::Hungry;
        }
        if self.happiness < 20.0 {
            return Mood::Sad;
        }

        let avg = (self.hunger + self.happiness + self.energy) / 3.0;
        if avg >= 80.0 {
            Mood::Ecstatic
        } else if avg >= 60.0 {
            Mood::Happy
        } else if avg >= 40.0 {
            Mood::Content
        } else {
            Mood::Bored
        }
    }
}

use std::path::PathBuf;

fn pet_state_path() -> PathBuf {
    let dir = std::env::var("TABBY_ZJ_STATE_DIR").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        format!("{}/.local/state/tabby-zj", home)
    });
    PathBuf::from(dir).join("pet.json")
}

pub fn save_pet(state: &PetState) {
    let path = pet_state_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(&path, json);
    }
}

pub fn load_pet() -> Option<PetState> {
    let path = pet_state_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_pet() {
        let p = PetState::default();
        assert_eq!(p.name, "Whiskers");
        assert_eq!(p.hunger, 80.0);
        assert_eq!(p.happiness, 80.0);
        assert_eq!(p.energy, 100.0);
        assert_eq!(p.mood, Mood::Content);
        assert!(!p.is_sleeping);
    }

    #[test]
    fn test_new_with_name() {
        let p = PetState::new("Luna", 42);
        assert_eq!(p.name, "Luna");
        assert_eq!(p.birth_tick, 42);
    }

    #[test]
    fn test_tick_decays_stats() {
        let mut p = PetState::default();
        let h0 = p.hunger;
        let e0 = p.energy;
        p.tick();
        assert!(p.hunger < h0);
        assert!(p.energy < e0);
        assert_eq!(p.tick, 1);
    }

    #[test]
    fn test_feed_increases_hunger() {
        let mut p = PetState::default();
        p.hunger = 40.0;
        p.feed();
        assert_eq!(p.hunger, 70.0);
        assert_eq!(p.last_fed_tick, p.tick);
    }

    #[test]
    fn test_feed_caps_at_100() {
        let mut p = PetState::default();
        p.hunger = 90.0;
        p.feed();
        assert_eq!(p.hunger, 100.0);
    }

    #[test]
    fn test_pet_increases_happiness() {
        let mut p = PetState::default();
        p.happiness = 50.0;
        p.pet();
        assert_eq!(p.happiness, 65.0);
    }

    #[test]
    fn test_play_costs_energy() {
        let mut p = PetState::default();
        p.energy = 50.0;
        p.happiness = 50.0;
        p.hunger = 50.0;
        p.play();
        assert_eq!(p.energy, 40.0);
        assert_eq!(p.happiness, 70.0);
        assert_eq!(p.hunger, 45.0);
    }

    #[test]
    fn test_sleeping_blocks_actions() {
        let mut p = PetState::default();
        p.is_sleeping = true;
        p.hunger = 50.0;
        p.happiness = 50.0;
        p.feed();
        p.pet();
        p.play();
        assert_eq!(p.hunger, 50.0);
        assert_eq!(p.happiness, 50.0);
    }

    #[test]
    fn test_auto_sleep_when_exhausted() {
        let mut p = PetState::default();
        p.energy = 5.0;
        p.tick();
        assert!(p.is_sleeping);
        assert_eq!(p.mood, Mood::Sleeping);
    }

    #[test]
    fn test_auto_wake_when_rested() {
        let mut p = PetState::default();
        p.is_sleeping = true;
        p.energy = 95.0;
        p.tick();
        assert!(!p.is_sleeping);
    }

    #[test]
    fn test_mood_hungry_when_starving() {
        let mut p = PetState::default();
        p.hunger = 10.0;
        p.happiness = 80.0;
        p.energy = 80.0;
        p.tick();
        assert_eq!(p.mood, Mood::Hungry);
    }

    #[test]
    fn test_mood_sad_when_unhappy() {
        let mut p = PetState::default();
        p.hunger = 80.0;
        p.happiness = 10.0;
        p.energy = 80.0;
        p.tick();
        assert_eq!(p.mood, Mood::Sad);
    }

    #[test]
    fn test_mood_ecstatic_when_all_high() {
        let mut p = PetState::default();
        p.hunger = 90.0;
        p.happiness = 90.0;
        p.energy = 90.0;
        p.tick();
        assert_eq!(p.mood, Mood::Ecstatic);
    }

    #[test]
    fn test_mood_bored_when_middling() {
        let mut p = PetState::default();
        p.hunger = 30.0;
        p.happiness = 30.0;
        p.energy = 30.0;
        p.tick();
        assert_eq!(p.mood, Mood::Bored);
    }

    #[test]
    fn test_stats_dont_go_below_zero() {
        let mut p = PetState::default();
        p.hunger = 0.0;
        p.happiness = 0.0;
        p.energy = 0.0;
        p.is_sleeping = false;
        p.tick();
        assert_eq!(p.hunger, 0.0);
        assert_eq!(p.happiness, 0.0);
        assert_eq!(p.energy, 0.0);
    }

    #[test]
    fn test_sleeping_recovers_energy() {
        let mut p = PetState::default();
        p.is_sleeping = true;
        p.energy = 50.0;
        p.tick();
        assert_eq!(p.energy, 50.5);
    }

    #[test]
    fn test_many_ticks_hunger_drains() {
        let mut p = PetState::default();
        for _ in 0..1000 {
            p.tick();
        }
        assert!(
            p.hunger < 0.01,
            "hunger should be effectively zero: {}",
            p.hunger
        );
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut p = PetState::new("Mochi", 10);
        p.hunger = 55.5;
        p.happiness = 72.3;
        p.mood = Mood::Happy;
        let json = serde_json::to_string(&p).unwrap();
        let loaded: PetState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.name, "Mochi");
        assert_eq!(loaded.hunger, 55.5);
        assert_eq!(loaded.happiness, 72.3);
        assert_eq!(loaded.mood, Mood::Happy);
    }
}
