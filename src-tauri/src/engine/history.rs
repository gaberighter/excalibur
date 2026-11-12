//! Recent samples for sparklines, kept in memory only (D7, D29).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use serde::Serialize;

/// Every sparkline covers this much time, whatever the refresh rate.
pub const WINDOW: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Point {
    /// Milliseconds since the Unix epoch.
    pub time_ms: u64,
    /// `None` is a gap: the plugin had no value at this moment.
    pub value: Option<f64>,
}

#[derive(Debug, Default)]
pub struct History {
    points: VecDeque<(Instant, Point)>,
}

impl History {
    pub fn push(&mut self, at: Instant, point: Point) {
        self.points.push_back((at, point));
        self.prune(at);
    }

    /// Drops points older than the window, measured back from `now`.
    pub fn prune(&mut self, now: Instant) {
        while let Some((at, _)) = self.points.front()
            && now.saturating_duration_since(*at) > WINDOW
        {
            self.points.pop_front();
        }
    }

    pub fn points(&self) -> Vec<Point> {
        self.points.iter().map(|(_, point)| *point).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(time_ms: u64, value: f64) -> Point {
        Point {
            time_ms,
            value: Some(value),
        }
    }

    #[test]
    fn keeps_points_in_order() {
        let mut history = History::default();
        let start = Instant::now();
        history.push(start, point(0, 1.0));
        history.push(start + Duration::from_secs(1), point(1000, 2.0));
        assert_eq!(history.points(), [point(0, 1.0), point(1000, 2.0)]);
    }

    #[test]
    fn drops_points_older_than_the_window() {
        let mut history = History::default();
        let start = Instant::now();
        history.push(start, point(0, 1.0));
        history.push(start + Duration::from_secs(30), point(30_000, 2.0));
        history.push(start + Duration::from_secs(61), point(61_000, 3.0));
        assert_eq!(history.points(), [point(30_000, 2.0), point(61_000, 3.0)]);
    }

    #[test]
    fn a_point_exactly_at_the_window_edge_is_kept() {
        let mut history = History::default();
        let start = Instant::now();
        history.push(start, point(0, 1.0));
        history.push(start + WINDOW, point(60_000, 2.0));
        assert_eq!(history.points().len(), 2);
    }

    #[test]
    fn gaps_are_recorded() {
        let mut history = History::default();
        let gap = Point {
            time_ms: 5,
            value: None,
        };
        history.push(Instant::now(), gap);
        assert_eq!(history.points(), [gap]);
    }
}
