// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use lucipane_lib::cli::{self, Command};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::parse(&args) {
        // No command: start the overlay.
        Ok(None) => lucipane_lib::run(None),
        // `clear` has to reach the running overlay, which happens through the
        // single-instance plugin inside the app startup.
        Ok(Some(Command::Clear)) => lucipane_lib::run(Some(Command::Clear)),
        // Everything else runs right here as a plain CLI.
        Ok(Some(command)) => std::process::exit(cli::run(command)),
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    }
}
