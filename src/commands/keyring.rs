use std::io;

use crate::console::log;
use crossterm::{
    cursor::MoveUp,
    execute,
    terminal::{Clear, ClearType},
};
use keyring::Entry;

#[cfg(windows)]
const LINE_ENDING: &'static str = "\r\n";
#[cfg(not(windows))]
const LINE_ENDING: &'static str = "\n";

pub fn store(service: &str, name: &str) {
  // Checking if a password already exists
  let keyring_entry = Entry::new(&service, &name).unwrap();

  if let Ok(_) = keyring_entry.get_password() {
      log::panic!("A password already exists for service {service} with name {name}, you must clear before to continue");
  }

  // Getting secret from user
  let mut input = String::new();
  log::info!("Enter your secret: ");
  io::stdin()
      .read_line(&mut input)
      .expect("error: unable to read user input");

  keyring_entry.set_password(&input.trim_end_matches(LINE_ENDING)).unwrap();

  // Remove secret displayed
  let mut stdout = std::io::stdout();
  execute!(stdout, MoveUp(2)).unwrap();
  execute!(stdout, Clear(ClearType::FromCursorDown)).unwrap();

  log::success!("You secret has been stored (service={service} name={name}) !");
}

pub fn clear(service: &str, name: &str) {
    let keyring_entry = Entry::new(&service, &name).unwrap();
    keyring_entry.delete_credential().unwrap();
    log::success!("You secret has been cleared (service={service} name={name}) !");
}
