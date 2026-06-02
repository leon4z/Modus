// Purpose: Desktop application binary entrypoint.

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    modus_lib::run()
}
