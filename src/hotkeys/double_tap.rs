use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct DoubleTap {
    pressed: Option<Instant>,
    first: Option<Instant>,
}

impl DoubleTap {
    pub(super) fn key(&mut self, matches: bool, down: bool, now: Instant) -> bool {
        if !matches {
            self.pressed = None;
            self.first = None;
            return false;
        }
        if down {
            if self.pressed.is_none() {
                self.pressed = Some(now);
            }
            return false;
        }
        let Some(pressed) = self.pressed.take() else {
            return false;
        };
        if now.duration_since(pressed) >= Duration::from_millis(300) {
            self.first = None;
            return false;
        }
        if self
            .first
            .is_some_and(|first| now.duration_since(first) <= Duration::from_millis(350))
        {
            self.first = None;
            true
        } else {
            self.first = Some(pressed);
            false
        }
    }
}
