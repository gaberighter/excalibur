// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;

const DEFAULT_PRINT_SECS: u64 = 10;

fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--print-samples") {
        let secs = args.next().and_then(|s| s.parse().ok());
        excalibur_lib::print_samples(Duration::from_secs(secs.unwrap_or(DEFAULT_PRINT_SECS)));
        return;
    }
    excalibur_lib::run();
}
