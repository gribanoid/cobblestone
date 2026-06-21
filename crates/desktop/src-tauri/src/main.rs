// SPDX-License-Identifier: MIT

// Prevents a terminal window from appearing on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    cobblestone_tauri::run();
}
