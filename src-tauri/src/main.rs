//! 바이너리 entrypoint. 실제 로직은 lib(`axon_lib::run`)에 있다.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    axon_lib::run()
}
