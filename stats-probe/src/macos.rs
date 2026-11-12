//! Metrics read through macOS system calls. None of these need privileges.

use std::mem;

use crate::reading::Reading;

const BYTES_PER_GIB: f64 = 1024.0 * 1024.0 * 1024.0;
const BYTES_PER_GB: f64 = 1e9;

/// Cumulative busy time averaged across cores, in percent-seconds, a counter
/// (D33). Its change per second of wall-clock time is CPU usage in percent.
pub fn cpu() -> Reading {
    let mut info: libc::host_cpu_load_info = unsafe { mem::zeroed() };
    let mut count = libc::HOST_CPU_LOAD_INFO_COUNT;
    // SAFETY: `info` is a valid host_cpu_load_info and `count` is its size in
    // integer_t units, as host_statistics requires.
    #[allow(deprecated)]
    let status = unsafe {
        libc::host_statistics(
            libc::mach_host_self(),
            libc::HOST_CPU_LOAD_INFO,
            &mut info as *mut _ as libc::host_info_t,
            &mut count,
        )
    };
    if status != libc::KERN_SUCCESS {
        return Reading::unavailable("could not read CPU statistics");
    }
    let [user, system, _idle, nice] = info.cpu_ticks;
    let busy_ticks = user as u64 + system as u64 + nice as u64;
    // SAFETY: sysconf has no preconditions.
    let (ticks_per_second, cores) = unsafe {
        (
            libc::sysconf(libc::_SC_CLK_TCK),
            libc::sysconf(libc::_SC_NPROCESSORS_ONLN),
        )
    };
    if ticks_per_second <= 0 || cores <= 0 {
        return Reading::unavailable("could not read CPU tick rate or core count");
    }
    Reading::value(percent_seconds(
        busy_ticks,
        ticks_per_second as f64,
        cores as f64,
    ))
}

/// `busy_ticks` is summed over all cores.
fn percent_seconds(busy_ticks: u64, ticks_per_second: f64, cores: f64) -> f64 {
    busy_ticks as f64 / ticks_per_second / cores * 100.0
}

/// Memory in use out of the memory installed, in GB (D37): app memory +
/// wired + compressed, the same sum Activity Monitor shows as "Memory Used".
pub fn memory() -> Reading {
    let mut stats: libc::vm_statistics64 = unsafe { mem::zeroed() };
    let mut count = libc::HOST_VM_INFO64_COUNT;
    // SAFETY: `stats` is a valid vm_statistics64 and `count` is its size in
    // integer_t units, as host_statistics64 requires.
    #[allow(deprecated)]
    let status = unsafe {
        libc::host_statistics64(
            libc::mach_host_self(),
            libc::HOST_VM_INFO64,
            &mut stats as *mut _ as libc::host_info64_t,
            &mut count,
        )
    };
    if status != libc::KERN_SUCCESS {
        return Reading::unavailable("could not read memory statistics");
    }
    // SAFETY: sysconf has no preconditions.
    let (page_size, installed_pages) = unsafe {
        (
            libc::sysconf(libc::_SC_PAGESIZE),
            libc::sysconf(libc::_SC_PHYS_PAGES),
        )
    };
    if page_size <= 0 {
        return Reading::unavailable("could not read memory page size");
    }
    let gib = |pages: u64| pages as f64 * page_size as f64 / BYTES_PER_GIB;
    let app = (stats.internal_page_count as u64).saturating_sub(stats.purgeable_count as u64);
    let used_pages = app + stats.wire_count as u64 + stats.compressor_page_count as u64;
    if installed_pages <= 0 {
        return Reading::value(gib(used_pages));
    }
    Reading::value_of(gib(used_pages), gib(installed_pages as u64))
}

/// Free space out of the size of the startup volume, in GB (D37).
pub fn storage() -> Reading {
    let mut stats: libc::statvfs = unsafe { mem::zeroed() };
    // SAFETY: the path is a NUL-terminated string and `stats` is a valid statvfs.
    let status = unsafe { libc::statvfs(c"/".as_ptr(), &mut stats) };
    if status != 0 {
        return Reading::unavailable("could not read disk statistics");
    }
    let block_size = stats.f_frsize as f64;
    let free_bytes = stats.f_bavail as f64 * block_size;
    let total_bytes = stats.f_blocks as f64 * block_size;
    Reading::value_of(free_bytes / BYTES_PER_GB, total_bytes / BYTES_PER_GB)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_fully_busy_second_on_every_core_is_100_percent_seconds() {
        // 8 cores at 100 ticks/s, all busy for one second
        assert_eq!(percent_seconds(800, 100.0, 8.0), 100.0);
    }

    #[test]
    fn one_busy_core_out_of_eight_is_an_eighth() {
        assert_eq!(percent_seconds(100, 100.0, 8.0), 12.5);
    }
}
