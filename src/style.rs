use std::env;
use std::sync::OnceLock;

/// ANSI styling resolved once per process. All fields are empty strings when
/// color is disabled, so the same format strings work in both modes.
pub struct Style {
    pub reset: &'static str,
    pub bold: &'static str,
    pub red: &'static str,
    pub yellow: &'static str,
    pub green: &'static str,
}

const COLOR: Style = Style {
    reset: "\x1b[00m",
    bold: "\x1b[1m",
    red: "\x1b[01;31m",
    yellow: "\x1b[01;33m",
    green: "\x1b[01;32m",
};

const PLAIN: Style = Style {
    reset: "",
    bold: "",
    red: "",
    yellow: "",
    green: "",
};

/// Return the active palette. Color is enabled only when `NO_COLOR` is unset
/// and stdout is a terminal. The decision is made once and cached.
pub fn active() -> &'static Style {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    let enabled = *ENABLED.get_or_init(|| env::var_os("NO_COLOR").is_none() && stdout_is_tty());
    if enabled { &COLOR } else { &PLAIN }
}

fn stdout_is_tty() -> bool {
    unsafe extern "C" {
        fn isatty(fd: i32) -> i32;
    }
    // File descriptor 1 is stdout.
    unsafe { isatty(1) == 1 }
}
