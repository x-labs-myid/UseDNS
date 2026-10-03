use std::time::{Duration, Instant};

/// Delay only when crossing the gap between the icon and the preview.
#[derive(Default)]
pub struct HoverDismissal {
    outside_since: Option<Instant>,
}

impl HoverDismissal {
    pub fn reset(&mut self) {
        self.outside_since = None;
    }

    pub fn should_hide(&mut self, now: Instant, over_icon: bool, over_preview: bool) -> bool {
        if over_icon || over_preview {
            self.reset();
            false
        } else {
            now.duration_since(*self.outside_since.get_or_insert(now)) >= Duration::from_millis(150)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn survives_crossing_gap_and_hides_after_leaving_both() {
        let now = Instant::now();
        let mut state = HoverDismissal::default();
        assert!(!state.should_hide(now, true, false));
        assert!(!state.should_hide(now, false, false));
        assert!(!state.should_hide(now + Duration::from_millis(100), false, true));
        assert!(!state.should_hide(now + Duration::from_millis(400), false, true));
        assert!(!state.should_hide(now + Duration::from_millis(500), false, false));
        assert!(state.should_hide(now + Duration::from_millis(650), false, false));
        assert!(!state.should_hide(now + Duration::from_millis(700), true, false));
    }
}
