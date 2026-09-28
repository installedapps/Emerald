use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlinkCursor {
    interval: Duration,
    visible: bool,
}

impl BlinkCursor {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            visible: true,
        }
    }

    pub fn tick(&mut self) {
        self.visible = !self.visible;
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn show(&mut self) {
        self.visible = true;
    }
}

impl Default for BlinkCursor {
    fn default() -> Self {
        Self::new(Duration::from_millis(530))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_toggles_visibility_on_tick() {
        let mut cursor = BlinkCursor::default();

        assert!(cursor.visible());
        cursor.tick();
        assert!(!cursor.visible());
        cursor.tick();
        assert!(cursor.visible());
    }

    #[test]
    fn cursor_uses_a_human_readable_blink_interval() {
        let cursor = BlinkCursor::default();

        assert_eq!(cursor.interval(), Duration::from_millis(530));
    }
}
