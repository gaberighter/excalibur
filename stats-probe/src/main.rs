use std::process::ExitCode;

fn main() -> ExitCode {
    // Subcommands (one per built-in metric) are added in phase 2.
    match std::env::args().nth(1) {
        Some(metric) => eprintln!("stats-probe: unknown metric '{metric}'"),
        None => eprintln!("usage: stats-probe <metric>"),
    }
    ExitCode::from(2)
}
