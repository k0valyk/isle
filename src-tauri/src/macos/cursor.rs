//! macOS cursor bridge.
//!
//! The island leans towards the pointer on Windows; on macOS the global cursor
//! position needs a CGEvent callback, so the lean stays neutral for now.

use tauri::AppHandle;

pub fn init(_app: AppHandle) {
    // Reserved for a CGEvent-based cursor feed.
}
