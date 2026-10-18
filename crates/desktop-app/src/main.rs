// SPDX-License-Identifier: MIT

fn main() {
    let store = cobblestone_core::Store::init().expect("Failed to open Cobblestone vault");
    cobblestone_ui::launch_desktop(store);
}
