//! macOS has no equivalent one-time elevation task; the app runs unelevated.

pub fn is_elevated() -> bool {
    true
}

pub fn task_exists() -> bool {
    false
}

pub fn enable() -> Result<(), String> {
    Err("Elevated access is only needed on Windows".into())
}

pub fn disable() -> bool {
    false
}

pub fn should_handoff() -> bool {
    false
}

pub fn handoff() {}
