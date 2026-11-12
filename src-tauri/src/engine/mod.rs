//! The plugin engine: finds plugins, runs them on a schedule and turns what
//! they print into samples. It knows nothing about the window, so it can be
//! tested without one.

use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;

pub mod command;
pub mod config;
pub mod discovery;
pub mod history;
pub mod manifest;
pub mod runner;
pub mod sample;

use config::Config;
use discovery::Plugin;
use history::{History, Point};
use manifest::{Display, Source};
use runner::{Exec, Output, Reading};
use sample::Tracker;

/// Where the engine looks for things on disk.
#[derive(Debug, Clone)]
pub struct Paths {
    /// Built-in plugins shipped with the app (D21).
    pub bundled_plugins: Option<PathBuf>,
    /// The user's folder, `~/.config/excalibur` (D20).
    pub user_dir: PathBuf,
    /// The `stats-probe` helper that collects `batch` plugins (D45).
    pub helper: Option<PathBuf>,
}

/// What the window needs to know to draw a plugin's row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub label: String,
    pub short: String,
    pub unit: String,
    pub display: Display,
    pub decimals: u8,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub builtin: bool,
    pub interval_secs: f64,
}

impl From<&Plugin> for PluginInfo {
    fn from(plugin: &Plugin) -> Self {
        let manifest = &plugin.manifest;
        PluginInfo {
            name: manifest.name.clone(),
            label: manifest.label.clone(),
            short: manifest.short.clone(),
            unit: manifest.unit.clone(),
            display: manifest.display,
            decimals: manifest.decimals,
            min: manifest.min,
            max: manifest.max,
            builtin: plugin.builtin,
            interval_secs: plugin.interval.as_secs_f64(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SampleEvent {
    pub plugin: String,
    /// Milliseconds since the Unix epoch.
    pub time_ms: u64,
    pub value: Option<f64>,
    /// Why there is no value, when there is none.
    pub reason: Option<String>,
    /// The scale the value sits on, if the plugin or its manifest gives one.
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Event {
    /// Plugins were loaded, at start or after a reload.
    Loaded {
        plugins: Vec<PluginInfo>,
        warnings: Vec<String>,
    },
    Sample(SampleEvent),
}

/// The engine's current state, for a window that has just opened.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Snapshot {
    pub plugins: Vec<PluginInfo>,
    pub warnings: Vec<String>,
    pub history: BTreeMap<String, Vec<Point>>,
}

#[derive(Default)]
struct Shared {
    plugins: Vec<PluginInfo>,
    warnings: Vec<String>,
    history: HashMap<String, History>,
}

enum Message {
    Stop,
    Reload,
    /// A worker finished running one plugin, or one batch of them.
    Done {
        generation: u64,
        readings: Vec<(usize, Output)>,
        at: Instant,
        time_ms: u64,
    },
}

pub struct Engine {
    sender: Sender<Message>,
    shared: Arc<Mutex<Shared>>,
    thread: Option<JoinHandle<()>>,
}

impl Engine {
    /// Loads plugins and starts running them. `on_event` is called from the
    /// engine's own thread.
    pub fn start(paths: Paths, on_event: impl Fn(Event) + Send + 'static) -> Engine {
        let (sender, receiver) = mpsc::channel();
        let shared = Arc::new(Mutex::new(Shared::default()));
        let scheduler = Scheduler {
            paths,
            sender: sender.clone(),
            receiver,
            shared: Arc::clone(&shared),
            on_event: Box::new(on_event),
            generation: 0,
        };
        let thread = thread::spawn(move || scheduler.run());
        Engine {
            sender,
            shared,
            thread: Some(thread),
        }
    }

    /// Re-reads config and plugin folders (D24).
    pub fn reload(&self) {
        let _ = self.sender.send(Message::Reload);
    }

    pub fn snapshot(&self) -> Snapshot {
        let shared = self.shared.lock().unwrap();
        Snapshot {
            plugins: shared.plugins.clone(),
            warnings: shared.warnings.clone(),
            history: shared
                .history
                .iter()
                .map(|(name, history)| (name.clone(), history.points()))
                .collect(),
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// One loaded plugin and its scheduling state.
struct Slot {
    plugin: Plugin,
    /// The resolved executable of a `command` plugin, if it was found.
    program: Option<PathBuf>,
    next_due: Instant,
    /// A run is in progress; ticks that fall due meanwhile are skipped (D28).
    in_flight: bool,
    tracker: Tracker,
}

struct Loaded {
    slots: Vec<Slot>,
    /// Intervals are counted from here, so plugins with the same interval
    /// fall due together and batch plugins share one run (D43).
    start: Instant,
    path_env: OsString,
    timeout: Duration,
}

struct Scheduler {
    paths: Paths,
    sender: Sender<Message>,
    receiver: Receiver<Message>,
    shared: Arc<Mutex<Shared>>,
    on_event: Box<dyn Fn(Event) + Send>,
    /// Bumped on reload, so results from before it are ignored.
    generation: u64,
}

const IDLE_WAIT: Duration = Duration::from_secs(3600);

impl Scheduler {
    fn run(mut self) {
        let mut loaded = self.load();
        loop {
            let now = Instant::now();
            self.dispatch_due(&mut loaded, now);
            let wait = loaded
                .slots
                .iter()
                .map(|slot| slot.next_due.saturating_duration_since(now))
                .min()
                .unwrap_or(IDLE_WAIT);
            match self.receiver.recv_timeout(wait) {
                Ok(Message::Stop) | Err(RecvTimeoutError::Disconnected) => break,
                Ok(Message::Reload) => {
                    self.generation += 1;
                    loaded = self.load();
                }
                Ok(Message::Done {
                    generation,
                    readings,
                    at,
                    time_ms,
                }) if generation == self.generation => {
                    for (index, reading) in readings {
                        self.record(&mut loaded, index, reading, at, time_ms);
                    }
                }
                Ok(Message::Done { .. }) | Err(RecvTimeoutError::Timeout) => {}
            }
        }
    }

    fn load(&self) -> Loaded {
        let (config, config_warning) = Config::load(&self.paths.user_dir.join(config::FILE_NAME));
        let discovered = discovery::discover(
            self.paths.bundled_plugins.as_deref(),
            &self.paths.user_dir.join("plugins"),
            &config,
        );
        let search_path = command::search_path(&config.path);
        let helper_dir = self.paths.helper.as_deref().and_then(|h| h.parent());
        let start = Instant::now();

        let slots: Vec<Slot> = discovered
            .plugins
            .into_iter()
            .map(|plugin| {
                let program = match &plugin.manifest.source {
                    Source::Command(command) => {
                        command::resolve(&command[0], &plugin.dir, helper_dir, &search_path)
                    }
                    Source::Batch(_) => None,
                };
                Slot {
                    plugin,
                    program,
                    next_due: start,
                    in_flight: false,
                    tracker: Tracker::default(),
                }
            })
            .collect();

        let plugins: Vec<PluginInfo> = slots.iter().map(|s| PluginInfo::from(&s.plugin)).collect();
        let warnings: Vec<String> = config_warning
            .into_iter()
            .chain(discovered.warnings)
            .collect();
        {
            let mut shared = self.shared.lock().unwrap();
            shared
                .history
                .retain(|name, _| plugins.iter().any(|p| &p.name == name));
            shared.plugins = plugins.clone();
            shared.warnings = warnings.clone();
        }
        (self.on_event)(Event::Loaded { plugins, warnings });

        Loaded {
            slots,
            start,
            path_env: command::join_path(&search_path),
            timeout: Duration::from_secs_f64(config.timeout),
        }
    }

    /// Starts a run for every plugin that is due and not already running.
    fn dispatch_due(&self, loaded: &mut Loaded, now: Instant) {
        let mut batch: Vec<(usize, String)> = Vec::new();
        for (index, slot) in loaded.slots.iter_mut().enumerate() {
            if slot.next_due > now {
                continue;
            }
            slot.next_due = next_due(loaded.start, slot.plugin.interval, now);
            if slot.in_flight {
                continue;
            }
            slot.in_flight = true;
            match &slot.plugin.manifest.source {
                Source::Batch(metric) => batch.push((index, metric.clone())),
                Source::Command(command) => {
                    let exec = slot.program.clone().map(|program| Exec {
                        program,
                        args: command[1..].to_vec(),
                        working_dir: Some(slot.plugin.dir.clone()),
                        path_env: loaded.path_env.clone(),
                        timeout: loaded.timeout,
                    });
                    let not_found = format!("command '{}' was not found", command[0]);
                    self.spawn_worker(move || {
                        let reading = match exec {
                            Some(exec) => runner::run_plugin(&exec),
                            None => Reading::Unavailable(not_found).into(),
                        };
                        vec![(index, reading)]
                    });
                }
            }
        }
        if !batch.is_empty() {
            let exec = self.paths.helper.clone().map(|program| Exec {
                program,
                args: Vec::new(),
                working_dir: None,
                path_env: loaded.path_env.clone(),
                timeout: loaded.timeout,
            });
            self.spawn_worker(move || run_batch(exec, batch));
        }
    }

    /// Runs `work` on its own thread and reports its readings back.
    fn spawn_worker(&self, work: impl FnOnce() -> Vec<(usize, Output)> + Send + 'static) {
        let sender = self.sender.clone();
        let generation = self.generation;
        thread::spawn(move || {
            let readings = work();
            let _ = sender.send(Message::Done {
                generation,
                readings,
                at: Instant::now(),
                time_ms: unix_millis(),
            });
        });
    }

    fn record(&self, loaded: &mut Loaded, index: usize, output: Output, at: Instant, time_ms: u64) {
        let Some(slot) = loaded.slots.get_mut(index) else {
            return;
        };
        slot.in_flight = false;
        let manifest = &slot.plugin.manifest;
        // Bounds reported with the reading win over the manifest's (D52).
        let min = output.bounds.min.or(manifest.min);
        let max = output.bounds.max.or(manifest.max);
        let sample = slot
            .tracker
            .update_within(manifest, output.reading, min, max, at);
        let point = Point {
            time_ms,
            value: sample.value,
        };
        {
            let mut shared = self.shared.lock().unwrap();
            shared
                .history
                .entry(manifest.name.clone())
                .or_default()
                .push(at, point);
            // So a window that loads later starts with the right scale.
            if let Some(info) = shared.plugins.get_mut(index) {
                info.min = min;
                info.max = max;
            }
        }
        (self.on_event)(Event::Sample(SampleEvent {
            plugin: manifest.name.clone(),
            time_ms,
            value: sample.value,
            reason: sample.reason,
            min,
            max,
        }));
    }
}

/// One helper run for all the batch plugins that are due.
fn run_batch(exec: Option<Exec>, batch: Vec<(usize, String)>) -> Vec<(usize, Output)> {
    let Some(exec) = exec else {
        let missing = Output::from(Reading::unavailable("the stats-probe helper was not found"));
        return batch
            .into_iter()
            .map(|(index, _)| (index, missing.clone()))
            .collect();
    };
    let mut metrics: Vec<String> = batch.iter().map(|(_, metric)| metric.clone()).collect();
    metrics.sort();
    metrics.dedup();
    let readings = runner::run_batch(&exec, &metrics);
    batch
        .into_iter()
        .map(|(index, metric)| (index, readings[&metric].clone()))
        .collect()
}

/// The first multiple of `interval` after `start` that is later than `now`.
fn next_due(start: Instant, interval: Duration, now: Instant) -> Instant {
    let elapsed = now.saturating_duration_since(start).as_secs_f64();
    let ticks = (elapsed / interval.as_secs_f64()).floor() as u32 + 1;
    let due = start + interval * ticks;
    if due > now { due } else { due + interval }
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64)
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::Path;

    use super::runner::test_support::script;
    use super::*;

    const WAIT: Duration = Duration::from_secs(10);

    /// A user folder with a fast refresh rate, and an engine running on it.
    struct Fixture {
        root: tempfile::TempDir,
        events: Receiver<Event>,
        engine: Option<Engine>,
    }

    impl Fixture {
        fn new(config: &str) -> Fixture {
            let root = tempfile::tempdir().unwrap();
            std::fs::write(
                root.path().join(config::FILE_NAME),
                format!("interval = 0.2\n{config}"),
            )
            .unwrap();
            let (_, events) = mpsc::channel();
            Fixture {
                root,
                events,
                engine: None,
            }
        }

        fn dir(&self) -> &Path {
            self.root.path()
        }

        /// Adds a drop-in plugin whose executable is the shell script `body`.
        fn add_plugin(&self, name: &str, manifest: &str, body: &str) {
            let dir = self.dir().join("plugins").join(name);
            script(&dir, "run", body);
            std::fs::write(
                dir.join(manifest::FILE_NAME),
                format!("command = [\"run\"]\n{manifest}"),
            )
            .unwrap();
        }

        fn add_batch_plugin(&self, name: &str, metric: &str) {
            let dir = self.dir().join("plugins").join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join(manifest::FILE_NAME),
                format!("batch = \"{metric}\""),
            )
            .unwrap();
        }

        fn start(&mut self, helper: Option<PathBuf>) {
            let (sender, events) = mpsc::channel();
            self.events = events;
            let paths = Paths {
                bundled_plugins: None,
                user_dir: self.dir().to_path_buf(),
                helper,
            };
            self.engine = Some(Engine::start(paths, move |event| {
                let _ = sender.send(event);
            }));
        }

        fn engine(&self) -> &Engine {
            self.engine.as_ref().unwrap()
        }

        /// The next `count` samples from `plugin`, ignoring other events.
        fn samples(&self, plugin: &str, count: usize) -> Vec<SampleEvent> {
            let deadline = Instant::now() + WAIT;
            let mut samples = Vec::new();
            while samples.len() < count {
                let left = deadline.saturating_duration_since(Instant::now());
                match self.events.recv_timeout(left) {
                    Ok(Event::Sample(sample)) if sample.plugin == plugin => samples.push(sample),
                    Ok(_) => {}
                    Err(_) => panic!("timed out waiting for samples from '{plugin}'"),
                }
            }
            samples
        }

        fn next_loaded(&self) -> (Vec<PluginInfo>, Vec<String>) {
            let deadline = Instant::now() + WAIT;
            loop {
                let left = deadline.saturating_duration_since(Instant::now());
                match self.events.recv_timeout(left) {
                    Ok(Event::Loaded { plugins, warnings }) => return (plugins, warnings),
                    Ok(_) => {}
                    Err(_) => panic!("timed out waiting for plugins to load"),
                }
            }
        }
    }

    const COUNT_UP: &str =
        r#"n=$(cat n 2>/dev/null || echo 0); n=$((n + 10)); echo $n > n; echo "{\"value\": $n}""#;

    #[test]
    fn gauge_plugin_produces_its_value() {
        let mut fixture = Fixture::new("");
        fixture.add_plugin("temp", "", r#"echo '{"value": 48.5}'"#);
        fixture.start(None);
        let samples = fixture.samples("temp", 2);
        assert!(
            samples
                .iter()
                .all(|s| s.value == Some(48.5) && s.reason.is_none())
        );
    }

    #[test]
    fn counter_plugin_has_no_value_first_then_a_rate() {
        let mut fixture = Fixture::new("");
        fixture.add_plugin("net", "kind = \"counter\"", COUNT_UP);
        fixture.start(None);
        let samples = fixture.samples("net", 3);
        assert_eq!(samples[0].value, None);
        assert!(samples[0].reason.is_some());
        // 10 more every 0.2 s is about 50 per second; timing is not exact.
        for sample in &samples[1..] {
            let rate = sample.value.expect("rate after the first reading");
            assert!((20.0..120.0).contains(&rate), "unexpected rate {rate}");
        }
    }

    #[test]
    fn failing_plugins_are_unavailable_and_do_not_affect_others() {
        let mut fixture = Fixture::new("");
        fixture.add_plugin("good", "", r#"echo '{"value": 1}'"#);
        fixture.add_plugin("exits", "", "exit 1");
        fixture.add_plugin("silent", "", "true");
        fixture.add_plugin("garbage", "", "echo not json");
        fixture.start(None);
        for broken in ["exits", "silent", "garbage"] {
            let sample = &fixture.samples(broken, 1)[0];
            assert_eq!(sample.value, None, "{broken}");
            assert!(sample.reason.is_some(), "{broken}");
        }
        assert_eq!(fixture.samples("good", 1)[0].value, Some(1.0));
    }

    #[test]
    fn missing_command_is_unavailable() {
        let mut fixture = Fixture::new("");
        let dir = fixture.dir().join("plugins/lost");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(manifest::FILE_NAME),
            "command = [\"no-such-tool-xyz\"]",
        )
        .unwrap();
        fixture.start(None);
        let sample = &fixture.samples("lost", 1)[0];
        assert!(sample.reason.as_deref().unwrap().contains("not found"));
    }

    #[test]
    fn slow_plugin_skips_ticks_instead_of_overlapping() {
        let mut fixture = Fixture::new("");
        // Takes 0.5 s but is due every 0.2 s.
        fixture.add_plugin(
            "slow",
            "",
            r#"echo start >> log; sleep 0.5; echo end >> log; echo '{"value": 1}'"#,
        );
        fixture.start(None);
        let started = Instant::now();
        fixture.samples("slow", 3);
        assert!(started.elapsed() >= Duration::from_millis(1400));

        let log = std::fs::read_to_string(fixture.dir().join("plugins/slow/log")).unwrap();
        let lines: Vec<&str> = log.lines().collect();
        for pair in lines.windows(2) {
            assert_ne!(pair, ["start", "start"], "two runs overlapped: {lines:?}");
        }
    }

    #[test]
    fn plugin_past_the_timeout_is_killed_and_unavailable() {
        let mut fixture = Fixture::new("timeout = 0.3");
        fixture.add_plugin("stuck", "", "sleep 30");
        fixture.start(None);
        let sample = &fixture.samples("stuck", 1)[0];
        assert_eq!(sample.value, None);
        assert!(sample.reason.as_deref().unwrap().contains("timed out"));
    }

    #[test]
    fn disabled_plugin_is_not_run() {
        let mut fixture = Fixture::new("disabled = [\"off\"]");
        fixture.add_plugin("off", "", r#"touch ran; echo '{"value": 1}'"#);
        fixture.add_plugin("on", "", r#"echo '{"value": 1}'"#);
        fixture.start(None);
        let (plugins, _) = fixture.next_loaded();
        assert_eq!(plugins.len(), 1);
        fixture.samples("on", 3);
        assert!(!fixture.dir().join("plugins/off/ran").exists());
    }

    #[test]
    fn batch_plugins_share_one_helper_run() {
        let mut fixture = Fixture::new("");
        let helper = script(
            &fixture.dir().join("bin"),
            "stats-probe",
            r#"echo "$*" >> "$(dirname "$0")/calls"
echo '{"cpu": {"value": 11}, "gpu": {"value": null, "reason": "no gpu"}}'"#,
        );
        fixture.add_batch_plugin("cpu", "cpu");
        fixture.add_batch_plugin("gpu", "gpu");
        fixture.add_batch_plugin("processor", "cpu");
        fixture.start(Some(helper));

        assert_eq!(fixture.samples("cpu", 1)[0].value, Some(11.0));
        assert_eq!(fixture.samples("processor", 1)[0].value, Some(11.0));
        let gpu = &fixture.samples("gpu", 1)[0];
        assert_eq!((gpu.value, gpu.reason.as_deref()), (None, Some("no gpu")));

        let calls = std::fs::read_to_string(fixture.dir().join("bin/calls")).unwrap();
        assert!(calls.lines().all(|line| line == "cpu gpu"), "{calls}");
    }

    #[test]
    fn batch_plugins_without_a_helper_are_unavailable() {
        let mut fixture = Fixture::new("");
        fixture.add_batch_plugin("cpu", "cpu");
        fixture.start(None);
        let sample = &fixture.samples("cpu", 1)[0];
        assert!(sample.reason.as_deref().unwrap().contains("helper"));
    }

    #[test]
    fn reload_picks_up_new_plugins_and_reports_warnings() {
        let mut fixture = Fixture::new("");
        fixture.add_plugin("first", "", r#"echo '{"value": 1}'"#);
        fixture.start(None);
        assert_eq!(fixture.next_loaded().0.len(), 1);

        fixture.add_plugin("second", "", r#"echo '{"value": 2}'"#);
        fixture.add_plugin("broken", "display = \"bar\"", "true");
        fixture.engine().reload();
        let (plugins, warnings) = fixture.next_loaded();
        let names: Vec<&str> = plugins.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["first", "second"]);
        assert_eq!(warnings.len(), 1);
        assert_eq!(fixture.samples("second", 1)[0].value, Some(2.0));
    }

    #[test]
    fn bounds_reported_by_a_plugin_override_the_manifest() {
        let mut fixture = Fixture::new("");
        fixture.add_plugin(
            "mem",
            "min = 0\nmax = 4",
            r#"echo '{"value": 5.8, "max": 8}'"#,
        );
        fixture.add_plugin("fixed", "min = 0\nmax = 4", r#"echo '{"value": 5.8}'"#);
        fixture.start(None);

        let mem = &fixture.samples("mem", 1)[0];
        assert_eq!(
            (mem.value, mem.min, mem.max),
            (Some(5.8), Some(0.0), Some(8.0))
        );
        // Without its own bounds, the manifest's apply and the value is clamped.
        let fixed = &fixture.samples("fixed", 1)[0];
        assert_eq!((fixed.value, fixed.max), (Some(4.0), Some(4.0)));

        let snapshot = fixture.engine().snapshot();
        let info = snapshot.plugins.iter().find(|p| p.name == "mem").unwrap();
        assert_eq!(info.max, Some(8.0));
    }

    #[test]
    fn snapshot_has_plugins_and_their_history() {
        let mut fixture = Fixture::new("");
        fixture.add_plugin(
            "temp",
            "unit = \"C\"\ndisplay = \"sparkline\"",
            r#"echo '{"value": 48.5}'"#,
        );
        fixture.start(None);
        fixture.samples("temp", 2);
        let snapshot = fixture.engine().snapshot();
        assert_eq!(snapshot.plugins[0].unit, "C");
        assert_eq!(snapshot.plugins[0].display, Display::Sparkline);
        assert!(snapshot.history["temp"].len() >= 2);
        assert_eq!(snapshot.history["temp"][0].value, Some(48.5));
    }

    #[test]
    fn next_due_is_the_following_multiple_of_the_interval() {
        let start = Instant::now();
        let interval = Duration::from_secs(2);
        let at = |secs: f64| start + Duration::from_secs_f64(secs);
        assert_eq!(next_due(start, interval, at(0.0)), at(2.0));
        assert_eq!(next_due(start, interval, at(0.5)), at(2.0));
        assert_eq!(next_due(start, interval, at(2.0)), at(4.0));
        assert_eq!(next_due(start, interval, at(5.9)), at(6.0));
    }

    #[test]
    fn plugins_with_different_intervals_fall_due_together() {
        let start = Instant::now();
        let now = start + Duration::from_millis(3100);
        let fast = next_due(start, Duration::from_secs(1), now);
        let slow = next_due(start, Duration::from_secs(2), now);
        assert_eq!(fast, slow);
    }
}
