use sysinfo::System;

pub fn is_process_running(process_name: &str) -> bool {
    let system = System::new_all();
    for _ in system.processes_by_name(std::ffi::OsStr::new(process_name)) {
        return true;
    }
    false
}
