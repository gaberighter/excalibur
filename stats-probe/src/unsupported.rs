//! Placeholder for operating systems without their own readers yet (D2).

use crate::reading::Reading;

const REASON: &str = "not supported on this operating system yet";

pub fn cpu() -> Reading {
    Reading::unavailable(REASON)
}

pub fn memory() -> Reading {
    Reading::unavailable(REASON)
}

pub fn storage() -> Reading {
    Reading::unavailable(REASON)
}
