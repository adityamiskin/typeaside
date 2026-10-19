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

#[cfg(test)]
mod tests {
    //! Failure modes: held keys, repeats, intervening keys, missing releases,
    //! timing boundaries, and a third tap accidentally firing twice.
    use super::*;

    #[test]
    fn detects_only_two_short_complete_taps() {
        for (events, expected) in [
            (
                vec![
                    (true, true, 0),
                    (true, false, 40),
                    (true, true, 100),
                    (true, false, 150),
                ],
                1,
            ),
            (
                vec![(true, true, 0), (true, true, 100), (true, false, 150)],
                0,
            ),
            (
                vec![
                    (true, true, 0),
                    (true, false, 300),
                    (true, true, 310),
                    (true, false, 340),
                ],
                0,
            ),
            (
                vec![
                    (true, true, 0),
                    (true, false, 40),
                    (true, true, 310),
                    (true, false, 351),
                ],
                0,
            ),
            (
                vec![
                    (true, true, 0),
                    (true, false, 40),
                    (true, true, 310),
                    (true, false, 350),
                ],
                1,
            ),
            (
                vec![
                    (true, true, 0),
                    (true, false, 40),
                    (false, true, 60),
                    (true, true, 100),
                    (true, false, 150),
                ],
                0,
            ),
            (
                vec![(true, false, 0), (true, true, 40), (true, false, 80)],
                0,
            ),
            (
                vec![
                    (true, true, 0),
                    (true, false, 40),
                    (true, true, 100),
                    (true, false, 150),
                    (true, true, 180),
                    (true, false, 200),
                ],
                1,
            ),
        ] {
            let start = Instant::now();
            let mut tap = DoubleTap::default();
            let count = events
                .into_iter()
                .filter(|&(key, down, ms)| tap.key(key, down, start + Duration::from_millis(ms)))
                .count();
            assert_eq!(count, expected);
        }
    }
}
