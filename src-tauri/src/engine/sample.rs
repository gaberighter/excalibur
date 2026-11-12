//! Turns raw readings into the values that are shown (D16, D46).

use std::time::Instant;

use super::manifest::{Kind, Manifest};
use super::runner::Reading;

/// What is shown for a plugin at one moment: a value, or why there is none.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub value: Option<f64>,
    pub reason: Option<String>,
}

impl Sample {
    fn value(value: f64) -> Self {
        Sample {
            value: Some(value),
            reason: None,
        }
    }

    fn none(reason: impl Into<String>) -> Self {
        Sample {
            value: None,
            reason: Some(reason.into()),
        }
    }
}

/// Remembers a counter's previous reading so its rate can be computed.
#[derive(Debug, Default)]
pub struct Tracker {
    previous: Option<(f64, Instant)>,
}

impl Tracker {
    /// Uses the bounds in the manifest.
    pub fn update(&mut self, manifest: &Manifest, reading: Reading, at: Instant) -> Sample {
        self.update_within(manifest, reading, manifest.min, manifest.max, at)
    }

    /// `min` and `max` are the bounds in force: the plugin's own from this
    /// run if it reported any, otherwise the manifest's.
    pub fn update_within(
        &mut self,
        manifest: &Manifest,
        reading: Reading,
        min: Option<f64>,
        max: Option<f64>,
        at: Instant,
    ) -> Sample {
        let value = match reading {
            Reading::Unavailable(reason) => {
                self.previous = None;
                return Sample::none(reason);
            }
            Reading::Value(value) => value,
        };
        let shown = match manifest.kind {
            Kind::Gauge => value,
            Kind::Counter => {
                let previous = self.previous.replace((value, at));
                match rate(previous, value, at) {
                    Ok(rate) => rate,
                    Err(reason) => return Sample::none(reason),
                }
            }
        };
        Sample::value(clamp(shown, min, max))
    }
}

/// Change per second since the previous reading.
fn rate(previous: Option<(f64, Instant)>, value: f64, at: Instant) -> Result<f64, &'static str> {
    let Some((previous_value, previous_at)) = previous else {
        return Err("waiting for a second reading");
    };
    let elapsed = at.saturating_duration_since(previous_at).as_secs_f64();
    if value < previous_value {
        // The counter wrapped or was reset, for example after a restart.
        return Err("counter was reset");
    }
    if elapsed <= 0.0 {
        return Err("readings were too close together");
    }
    Ok((value - previous_value) / elapsed)
}

fn clamp(value: f64, min: Option<f64>, max: Option<f64>) -> f64 {
    let value = min.map_or(value, |min| value.max(min));
    max.map_or(value, |max| value.min(max))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn manifest(extra: &str) -> Manifest {
        Manifest::parse(&format!("command = [\"run\"]\n{extra}"), "test").unwrap()
    }

    #[test]
    fn gauge_value_passes_through() {
        let mut tracker = Tracker::default();
        let sample = tracker.update(&manifest(""), Reading::Value(48.0), Instant::now());
        assert_eq!(sample, Sample::value(48.0));
    }

    #[test]
    fn unavailable_reading_keeps_its_reason() {
        let mut tracker = Tracker::default();
        let reading = Reading::unavailable("sensor not found");
        let sample = tracker.update(&manifest(""), reading, Instant::now());
        assert_eq!(sample, Sample::none("sensor not found"));
    }

    #[test]
    fn counter_has_no_value_until_the_second_reading() {
        let counter = manifest("kind = \"counter\"");
        let mut tracker = Tracker::default();
        let start = Instant::now();
        let first = tracker.update(&counter, Reading::Value(1000.0), start);
        assert_eq!(first.value, None);

        let later = start + Duration::from_secs(2);
        let second = tracker.update(&counter, Reading::Value(1047.0), later);
        assert_eq!(second, Sample::value(23.5));
    }

    #[test]
    fn counter_going_backwards_gives_no_value_then_recovers() {
        let counter = manifest("kind = \"counter\"");
        let mut tracker = Tracker::default();
        let start = Instant::now();
        tracker.update(&counter, Reading::Value(1000.0), start);

        let reset = tracker.update(
            &counter,
            Reading::Value(5.0),
            start + Duration::from_secs(1),
        );
        assert_eq!(reset.value, None);

        let next = tracker.update(
            &counter,
            Reading::Value(15.0),
            start + Duration::from_secs(2),
        );
        assert_eq!(next, Sample::value(10.0));
    }

    #[test]
    fn counter_starts_again_after_being_unavailable() {
        let counter = manifest("kind = \"counter\"");
        let mut tracker = Tracker::default();
        let start = Instant::now();
        tracker.update(&counter, Reading::Value(100.0), start);
        tracker.update(
            &counter,
            Reading::unavailable("gone"),
            start + Duration::from_secs(1),
        );

        let after = tracker.update(
            &counter,
            Reading::Value(900.0),
            start + Duration::from_secs(2),
        );
        assert_eq!(after.value, None);
    }

    #[test]
    fn values_are_clamped_to_min_and_max() {
        let bounded = manifest("min = 0\nmax = 100");
        let mut tracker = Tracker::default();
        let now = Instant::now();
        assert_eq!(
            tracker.update(&bounded, Reading::Value(100.4), now),
            Sample::value(100.0)
        );
        assert_eq!(
            tracker.update(&bounded, Reading::Value(-0.2), now),
            Sample::value(0.0)
        );
        assert_eq!(
            tracker.update(&bounded, Reading::Value(55.0), now),
            Sample::value(55.0)
        );
    }

    #[test]
    fn counter_rate_is_clamped() {
        let counter = manifest("kind = \"counter\"\nmax = 100");
        let mut tracker = Tracker::default();
        let start = Instant::now();
        tracker.update(&counter, Reading::Value(0.0), start);
        let sample = tracker.update(
            &counter,
            Reading::Value(101.0),
            start + Duration::from_secs(1),
        );
        assert_eq!(sample, Sample::value(100.0));
    }
}
