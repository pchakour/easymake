// Global variable
static LOG_LEVEL: AtomicUsize = AtomicUsize::new(0); // Default = Info

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Console,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    pub fn from_str(v: &str) -> Self {
        match v {
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            "trace" => LogLevel::Trace,
            _ => LogLevel::Console,
        }
    }

    pub fn as_usize(self) -> usize {
        match self {
            LogLevel::Console => 0,
            LogLevel::Info => 1,
            LogLevel::Debug => 2,
            LogLevel::Trace => 3,
        }
    }

    pub fn from_usize(v: usize) -> Self {
        match v {
            1 => LogLevel::Info,
            2 => LogLevel::Debug,
            3 => LogLevel::Trace,
            _ => LogLevel::Console,
        }
    }
}

pub fn set_log_level(level: LogLevel) {
    LOG_LEVEL.store(level.as_usize(), Ordering::Relaxed);
}

pub fn get_log_level() -> LogLevel {
    LogLevel::from_usize(LOG_LEVEL.load(Ordering::Relaxed))
}

pub const INDENT: &str = "   ";

pub enum StepStatus {
    Finished,
    Running,
    Skipped,
}

impl fmt::Display for StepStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StepStatus::Skipped => write!(f, "\x1b[1;90mSkipped\x1b[0m"), // gray
            StepStatus::Finished => write!(f, "\x1b[1;32mFinished\x1b[0m"), // green
            StepStatus::Running => write!(f, "\x1b[1;32mRunning\x1b[0m"), // green
        }
    }
}

#[allow(unused)]
macro_rules! step_info {
    // `()` indicates that the macro takes no argument.
    ($step_id:expr, $status:expr, $($arg:tt)*) => {{
        if log::LogLevel::as_usize(log::get_log_level()) > 0 {
            // The macro will expand into the contents of this block.
            log::info!("[\x1b[90m{}\x1b[0m] {} {}", $step_id, $status, $($arg)*);
        } else {
            log::info!("{} {}", $status, $($arg)*);
        }
    }};
}

#[allow(unused)]
macro_rules! action_info {
    // `()` indicates that the macro takes no argument.
    ($step_id:expr, $action_id:expr, $($arg:tt)*) => {{
        if log::LogLevel::as_usize(log::get_log_level()) > 0 {
            // The macro will expand into the contents of this block.
            log::info!("[\x1b[90m{}\x1b[0m] \x1b[1;34mAction {}\x1b[0m {}", $step_id, $action_id, format!($($arg)*));
        } else {
            log::info!("\x1b[1;34mAction {}\x1b[0m [\x1b[90m{}\x1b[0m] {}", $action_id, $step_id, format!($($arg)*));
        }
    }};
}

#[allow(unused)]
macro_rules! info {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {{
        if log::LogLevel::as_usize(log::get_log_level()) > 0 {
            // The macro will expand into the contents of this block.
            log::timestamp!("[\x1b[32minfo\x1b[0m] {}", format!($($arg)*));
        } else {
            log::text!($($arg)*);
        }
    }};
}

#[allow(unused)]
macro_rules! text {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {
        // The macro will expand into the contents of this block.
        println!("{}", format!($($arg)*));
    };
}

#[allow(unused)]
macro_rules! success {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {{
        log::info!("\n\x1b[32m{} {}\x1b[0m", "🎉", format!($($arg)*));
    }};
}

#[allow(unused)]
macro_rules! warning {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {
        // The macro will expand into the contents of this block.
        log::timestamp!("[\x1b[33mwarning\x1b[0m] {}", format!($($arg)*));
    };
}

#[allow(unused)]
macro_rules! panic {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*, $exit:expr) => {
        // The macro will expand into the contents of this block.
        if log::LogLevel::as_usize(log::get_log_level()) > 0 {
            // The macro will expand into the contents of this block.
            log::timestamp!("[\x1b[31mfatal\x1b[0m] \x1b[31m{}\x1b[0m", format!($($arg)*));
        } else {
            log::text!("\x1b[31m{}\x1b[0m", format!($($arg)*));
        }
        $exit;
    };
}

#[allow(unused)]
macro_rules! trace {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {{
        if log::LogLevel::as_usize(log::get_log_level()) > 2 {
            // The macro will expand into the contents of this block.
            log::timestamp!("[\x1b[2mtrace\x1b[0m] {}", format!($($arg)*));
        }
    }};
}

#[allow(unused)]
macro_rules! debug {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {{
        if log::LogLevel::as_usize(log::get_log_level()) > 1 {
            // The macro will expand into the contents of this block.
            log::timestamp!("[\x1b[36mdebug\x1b[0m] {}", format!($($arg)*));
        }
    }};
}

macro_rules! timestamp {
    ($($arg:tt)*) => {{
        let now = std::time::SystemTime::now();
        let dt: chrono::prelude::DateTime<chrono::prelude::Utc> = now.into();
        println!("[{}] {}", dt.format("%+"), format!($($arg)*));
    }}
}

use std::{
    fmt,
    sync::atomic::{AtomicUsize, Ordering},
};

#[allow(unused)]
pub(crate) use debug;
#[allow(unused)]
pub(crate) use info;
#[allow(unused)]
pub(crate) use panic;
#[allow(unused)]
pub(crate) use step_info;
#[allow(unused)]
pub(crate) use action_info;
#[allow(unused)]
pub(crate) use success;
#[allow(unused)]
pub(crate) use text;
#[allow(unused)]
pub(crate) use timestamp;
#[allow(unused)]
pub(crate) use trace;
#[allow(unused)]
pub(crate) use warning;
