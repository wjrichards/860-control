//! At most one update per interval while a control is dragged, and the last
//! value is always sent.

use std::time::{Duration, Instant};

pub struct Throttle<T> {
    interval: Duration,
    last_sent: Option<Instant>,
    held: Option<T>,
}

impl<T> Throttle<T> {
    pub fn new(interval: Duration) -> Self {
        Throttle { interval, last_sent: None, held: None }
    }

    /// A new value. Returns it if it should be sent now; otherwise it is held
    /// (replacing any older held value) until [`Self::flush`].
    pub fn push(&mut self, now: Instant, value: T) -> Option<T> {
        match self.last_sent {
            Some(t) if now.duration_since(t) < self.interval => {
                self.held = Some(value);
                None
            }
            _ => {
                self.last_sent = Some(now);
                self.held = None;
                Some(value)
            }
        }
    }

    /// Called when the interval since the last send has passed. Returns the
    /// held value, if any, to send now.
    pub fn flush(&mut self, now: Instant) -> Option<T> {
        let value = self.held.take()?;
        self.last_sent = Some(now);
        Some(value)
    }

    /// Whether the user is mid-change: something is held, or a value was sent
    /// within the last interval.
    pub fn busy(&self, now: Instant) -> bool {
        self.held.is_some() || self.last_sent.is_some_and(|t| now.duration_since(t) < self.interval)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);

    /// Drive the throttle like the GTK timer does: a flush at every interval
    /// boundary after a send.
    fn simulate(updates: &[(u64, u8)]) -> Vec<(u64, u8)> {
        let start = Instant::now();
        let mut t = Throttle::new(100 * MS);
        let mut sent = Vec::new();
        let mut next_flush: Option<u64> = None;
        let end = updates.last().unwrap().0 + 500;
        let mut i = 0;
        for ms in 0..=end {
            let now = start + ms as u32 * MS;
            if next_flush == Some(ms) {
                next_flush = None;
                if let Some(v) = t.flush(now) {
                    sent.push((ms, v));
                    next_flush = Some(ms + 100);
                }
            }
            while i < updates.len() && updates[i].0 == ms {
                if let Some(v) = t.push(now, updates[i].1) {
                    sent.push((ms, v));
                    next_flush = Some(ms + 100);
                } else if next_flush.is_none() {
                    next_flush = Some(ms + 100);
                }
                i += 1;
            }
        }
        sent
    }

    #[test]
    fn dragging_brightness_100_to_20_over_one_second() {
        // One update every 12.5 ms, as a fast drag produces.
        let updates: Vec<(u64, u8)> = (0..=80).map(|k| ((k * 25 / 2) as u64, (100 - k) as u8)).collect();
        let sent = simulate(&updates);
        // Changes span 0..=1000 ms, so one send per 100 ms allows 11 plus the
        // trailing final value: the spec's "about 10".
        assert!(sent.len() <= 12, "{} updates sent: {sent:?}", sent.len());
        assert_eq!(sent.last().unwrap().1, 20, "final value always sent");
        assert!(sent.windows(2).all(|w| w[1].0 - w[0].0 >= 100), "at most one per 100 ms: {sent:?}");
    }

    #[test]
    fn single_change_sent_immediately() {
        assert_eq!(simulate(&[(0, 42)]), [(0, 42)]);
    }

    #[test]
    fn busy_while_dragging() {
        let start = Instant::now();
        let mut t = Throttle::new(100 * MS);
        assert!(!t.busy(start));
        t.push(start, 1);
        assert!(t.busy(start + 50 * MS));
        assert!(!t.busy(start + 150 * MS));
        t.push(start + 160 * MS, 2);
        t.push(start + 170 * MS, 3);
        assert!(t.busy(start + 300 * MS), "a held value keeps it busy");
        assert_eq!(t.flush(start + 300 * MS), Some(3));
    }
}
