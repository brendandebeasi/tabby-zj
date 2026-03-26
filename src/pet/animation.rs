use super::Mood;

pub struct Animation {
    frames: &'static [&'static str],
    frame_index: usize,
    ticks_per_frame: u64,
    tick_counter: u64,
}

impl Animation {
    pub fn new(frames: &'static [&'static str], ticks_per_frame: u64) -> Self {
        Self {
            frames,
            frame_index: 0,
            ticks_per_frame,
            tick_counter: 0,
        }
    }

    pub fn tick(&mut self) {
        self.tick_counter += 1;
        if self.tick_counter >= self.ticks_per_frame {
            self.tick_counter = 0;
            self.frame_index = (self.frame_index + 1) % self.frames.len();
        }
    }

    pub fn current_frame(&self) -> &'static str {
        self.frames[self.frame_index]
    }

    pub fn set_mood(&mut self, mood: &Mood) {
        let new_frames = frames_for_mood(mood);
        if !std::ptr::eq(self.frames, new_frames) {
            self.frames = new_frames;
            self.frame_index = 0;
            self.tick_counter = 0;
        }
    }
}

static HAPPY_FRAMES: &[&str] = &["(=^◕ᴥ◕^=)", "(=^◕‿◕^=)ﾉ"];

static CONTENT_FRAMES: &[&str] = &["(=^·ω·^=)", "(=^·_·^=)"];

static BORED_FRAMES: &[&str] = &["(=^-_-^=)", "(=^· ·^=) ..."];

static HUNGRY_FRAMES: &[&str] = &["(=^;ω;^=)", "(=^; ;^=)"];

static SAD_FRAMES: &[&str] = &["(=^;_;^=)", "(=^; ;^=)"];

static SLEEPING_FRAMES: &[&str] = &["(=^-ω-^=) zzZ", "(=^- -^=) zZz"];

static ECSTATIC_FRAMES: &[&str] = &["\\(=^◕ᴥ◕^=)/", "(=^✧ω✧^=)"];

pub fn frames_for_mood(mood: &Mood) -> &'static [&'static str] {
    match mood {
        Mood::Ecstatic => ECSTATIC_FRAMES,
        Mood::Happy => HAPPY_FRAMES,
        Mood::Content => CONTENT_FRAMES,
        Mood::Bored => BORED_FRAMES,
        Mood::Hungry => HUNGRY_FRAMES,
        Mood::Sad => SAD_FRAMES,
        Mood::Sleeping => SLEEPING_FRAMES,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_advances_frame() {
        let mut anim = Animation::new(CONTENT_FRAMES, 2);
        assert_eq!(anim.current_frame(), CONTENT_FRAMES[0]);
        anim.tick();
        assert_eq!(anim.current_frame(), CONTENT_FRAMES[0]);
        anim.tick();
        assert_eq!(anim.current_frame(), CONTENT_FRAMES[1]);
    }

    #[test]
    fn test_animation_wraps_around() {
        let mut anim = Animation::new(CONTENT_FRAMES, 1);
        anim.tick();
        anim.tick();
        assert_eq!(anim.current_frame(), CONTENT_FRAMES[0]);
    }

    #[test]
    fn test_set_mood_resets_frame() {
        let mut anim = Animation::new(CONTENT_FRAMES, 1);
        anim.tick();
        assert_eq!(anim.frame_index, 1);
        anim.set_mood(&Mood::Happy);
        assert_eq!(anim.frame_index, 0);
        assert_eq!(anim.current_frame(), HAPPY_FRAMES[0]);
    }

    #[test]
    fn test_set_same_mood_no_reset() {
        let mut anim = Animation::new(CONTENT_FRAMES, 1);
        anim.tick();
        assert_eq!(anim.frame_index, 1);
        anim.set_mood(&Mood::Content);
        assert_eq!(anim.frame_index, 1);
    }

    #[test]
    fn test_all_moods_have_frames() {
        let moods = [
            Mood::Ecstatic,
            Mood::Happy,
            Mood::Content,
            Mood::Bored,
            Mood::Hungry,
            Mood::Sad,
            Mood::Sleeping,
        ];
        for mood in &moods {
            let frames = frames_for_mood(mood);
            assert!(frames.len() >= 2, "{:?} should have ≥2 frames", mood);
        }
    }

    #[test]
    fn test_frames_for_each_mood_unique() {
        assert_eq!(frames_for_mood(&Mood::Happy)[0], "(=^◕ᴥ◕^=)");
        assert_eq!(frames_for_mood(&Mood::Sleeping)[0], "(=^-ω-^=) zzZ");
        assert_eq!(frames_for_mood(&Mood::Hungry)[0], "(=^;ω;^=)");
    }
}
