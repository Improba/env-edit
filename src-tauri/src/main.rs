// Hide the console window for packaged Windows desktop builds.
// Keep it in debug builds so diagnostics remain visible during development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    envedit_lib::run();
}
