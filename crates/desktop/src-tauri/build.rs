// SPDX-License-Identifier: MIT

fn main() {
    println!("cargo:rerun-if-changed=tauri.conf.json");
    println!("cargo:rerun-if-changed=icons/cobblestone.svg");
    println!("cargo:rerun-if-changed=icons/icon.png");
    println!("cargo:rerun-if-changed=icons/icon.icns");
    tauri_build::build()
}
