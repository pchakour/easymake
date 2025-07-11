pub const INDENT: &str = "   ";

macro_rules! step {
    // `()` indicates that the macro takes no argument.
    ($current_step:tt, $total_step:tt, $($arg:tt)*) => {
        // The macro will expand into the contents of this block.
        // println!("\x1b[32m[{}/{}] {}\x1b[0m", $current_step, $total_step, format!($($arg)*));
    };
}

macro_rules! info {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {
        // The macro will expand into the contents of this block.
        println!("\x1b[34m{}\x1b[0m", format!($($arg)*));
    };
}

macro_rules! text {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {
        // The macro will expand into the contents of this block.
        println!("{}", format!($($arg)*));
    };
}

macro_rules! success {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {
        // The macro will expand into the contents of this block.
        println!("\n\x1b[32m{} {}\x1b[0m", "🎉", format!($($arg)*));
    };
}

macro_rules! warning {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {
        // The macro will expand into the contents of this block.
        println!("\x1b[33m{}\x1b[0m", format!($($arg)*));
    };
}
macro_rules! error {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {
        // The macro will expand into the contents of this block.
        println!("\x1b[31m{}\x1b[0m", format!($($arg)*));
    };
}

macro_rules! panic {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {
        // The macro will expand into the contents of this block.
        // panic!("\x1b[31m{}\x1b[0m", format!($($arg)*));
    };
}

pub(crate) use step;
pub(crate) use error;
pub(crate) use panic;
pub(crate) use warning;
pub(crate) use text;
pub(crate) use info;
pub(crate) use success;