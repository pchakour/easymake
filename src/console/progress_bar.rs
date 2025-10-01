use std::time::Duration;

use indicatif::{MultiProgress, MultiProgressAlignment, ProgressBar, ProgressStyle};
use once_cell::sync::Lazy;

use crate::console::log::{get_log_level, LogLevel};

static MP: Lazy<MultiProgress> = Lazy::new(|| {
    let m = MultiProgress::new();
    m.set_alignment(MultiProgressAlignment::Bottom); // sticky at bottom
    m
});

// Global sticky bar
static LOADER: Lazy<Option<ProgressBar>> = Lazy::new(|| {
  if get_log_level() == LogLevel::Console {
    let pb = MP.add(ProgressBar::new_spinner());
    pb.set_style(
        ProgressStyle::with_template("\n  {spinner:.cyan} {msg:.cyan.bold}")
            .unwrap()
            .tick_strings(&["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"]),
    );
    pb.enable_steady_tick(Duration::from_millis(80)); // spinner animates
    return Some(pb);
  }

    None
});

pub fn set_loader_message(msg: &str) {
  if let Some(progress) = &*LOADER {
    progress.set_message(String::from(msg));
  }
}

pub fn log_above_bar(msg: impl AsRef<str>) {
    // Prints a line above all progress bars (safe from any thread)
    // Ignored if output is not a tty (draw target hidden).
    let _ = MP.println(msg);
}

pub fn finish() {
  if let Some(progress) = &*LOADER {
    progress.finish_and_clear();
  }
}