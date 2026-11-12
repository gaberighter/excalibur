use std::cell::OnceCell;

use sysinfo::{Components, Gpus, Networks};

use crate::reading::Reading;

#[cfg(target_os = "macos")]
use crate::macos as os;
#[cfg(not(target_os = "macos"))]
use crate::unsupported as os;

/// Every metric this helper can report. Units are fixed per metric (D31, D36).
pub const NAMES: [&str; 7] = [
    "cpu",
    "memory",
    "storage",
    "temperature",
    "gpu",
    "net-down",
    "net-up",
];

const BYTES_PER_MB: f64 = 1e6;

/// Collects the metrics asked for in one run (D43). Sources shared by several
/// metrics are loaded once.
#[derive(Default)]
pub struct Probe {
    networks: OnceCell<Option<Networks>>,
}

impl Probe {
    /// Returns `None` for a name this helper does not know.
    pub fn read(&self, name: &str) -> Option<Reading> {
        Some(match name {
            "cpu" => os::cpu(),
            "memory" => os::memory(),
            "storage" => os::storage(),
            "temperature" => temperature(),
            "gpu" => gpu(),
            "net-down" => self.network(|n| n.total_received()),
            "net-up" => self.network(|n| n.total_transmitted()),
            _ => return None,
        })
    }

    /// Running total in MB across physical interfaces, a counter (D16).
    fn network(&self, bytes: impl Fn(&sysinfo::NetworkData) -> u64) -> Reading {
        let networks = self
            .networks
            .get_or_init(|| Networks::new_with_refreshed_list().ok());
        let Some(networks) = networks else {
            return Reading::unavailable("could not list network interfaces");
        };
        let mut total = None;
        for (name, data) in networks.iter() {
            if is_physical_interface(&name.to_string_lossy()) {
                *total.get_or_insert(0u64) += bytes(data);
            }
        }
        match total {
            Some(total) => Reading::value(total as f64 / BYTES_PER_MB),
            None => Reading::unavailable("no network interfaces found"),
        }
    }
}

/// Wi-Fi and Ethernet are `en*` on macOS and `en*`/`eth*`/`wl*` on Linux.
/// Loopback and VPN tunnels are left out so traffic is not counted twice.
fn is_physical_interface(name: &str) -> bool {
    ["en", "eth", "wl"].iter().any(|p| name.starts_with(p))
}

/// Average of the die sensors, in °C (D44).
fn temperature() -> Reading {
    let Ok(components) = Components::new_with_refreshed_list() else {
        return Reading::unavailable("could not read temperature sensors");
    };
    let sensors = components.iter().map(|c| (c.label(), c.temperature()));
    match average_die_temperature(sensors) {
        Some(avg) => Reading::value(avg),
        None => Reading::unavailable("no die temperature sensors found"),
    }
}

fn average_die_temperature<'a>(
    sensors: impl Iterator<Item = (&'a str, Option<f32>)>,
) -> Option<f64> {
    let (sum, count) = sensors
        .filter(|(label, _)| label.contains("tdie"))
        .filter_map(|(_, temp)| temp)
        .filter(|temp| temp.is_finite())
        .fold((0.0, 0u32), |(sum, count), temp| {
            (sum + temp as f64, count + 1)
        });
    (count > 0).then(|| sum / count as f64)
}

/// Instantaneous utilisation in percent, a gauge (D36).
fn gpu() -> Reading {
    let Ok(gpus) = Gpus::new_with_refreshed_list() else {
        return Reading::unavailable("could not list GPUs");
    };
    match gpus.iter().find_map(|gpu| gpu.usage()) {
        Some(usage) => Reading::value(usage as f64),
        None => Reading::unavailable("GPU does not report utilisation"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_name_is_none() {
        assert_eq!(Probe::default().read("battery"), None);
    }

    #[test]
    fn every_listed_name_is_known() {
        let probe = Probe::default();
        for name in NAMES {
            assert!(probe.read(name).is_some(), "{name} should be known");
        }
    }

    #[test]
    fn die_temperature_averages_only_die_sensors() {
        let sensors = [
            ("PMU tdie1", Some(30.0)),
            ("PMU2 tdie4", Some(40.0)),
            ("PMU tdev1", Some(-1.5)),
            ("gas gauge battery", Some(90.0)),
        ];
        assert_eq!(average_die_temperature(sensors.into_iter()), Some(35.0));
    }

    #[test]
    fn die_temperature_skips_missing_and_invalid_readings() {
        let sensors = [
            ("PMU tdie1", Some(30.0)),
            ("PMU tdie2", None),
            ("PMU tdie3", Some(f32::NAN)),
        ];
        assert_eq!(average_die_temperature(sensors.into_iter()), Some(30.0));
    }

    #[test]
    fn die_temperature_is_none_without_die_sensors() {
        let sensors = [("NAND CH0 temp", Some(27.0)), ("PMU tdie1", None)];
        assert_eq!(average_die_temperature(sensors.into_iter()), None);
    }

    #[test]
    fn physical_interfaces_exclude_loopback_and_tunnels() {
        assert!(is_physical_interface("en0"));
        assert!(!is_physical_interface("lo0"));
        assert!(!is_physical_interface("utun3"));
        assert!(!is_physical_interface("bridge0"));
    }
}
