pub const INDENT: &str = "   ";

#[allow(unused)]
macro_rules! step {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {
        // The macro will expand into the contents of this block.
        println!("\x1b[32m{}\x1b[0m", format!($($arg)*));
    };
}

#[allow(unused)]
macro_rules! info {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {
        // The macro will expand into the contents of this block.
        println!("\x1b[34m{}\x1b[0m", format!($($arg)*));
    };
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
    ($($arg:tt)*) => {
        // The macro will expand into the contents of this block.
        println!("\n\x1b[32m{} {}\x1b[0m", "🎉", format!($($arg)*));
    };
}

#[allow(unused)]
macro_rules! warning {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {
        // The macro will expand into the contents of this block.
        println!("\x1b[33m{}\x1b[0m", format!($($arg)*));
    };
}

#[allow(unused)]
macro_rules! error {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {
        // The macro will expand into the contents of this block.
        println!("\x1b[31m{}\x1b[0m", format!($($arg)*));
    };
}

#[allow(unused)]
macro_rules! panic {
    // `()` indicates that the macro takes no argument.
    ($($arg:tt)*) => {
        // The macro will expand into the contents of this block.
        // panic!("\x1b[31m{}\x1b[0m", format!($($arg)*));
    };
}

#[allow(unused)]
pub(crate) use step;
#[allow(unused)]
pub(crate) use error;
#[allow(unused)]
pub(crate) use panic;
#[allow(unused)]
pub(crate) use warning;
#[allow(unused)]
pub(crate) use text;
#[allow(unused)]
pub(crate) use info;
#[allow(unused)]
pub(crate) use success;