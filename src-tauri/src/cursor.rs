use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[link(name = "user32")]
extern "system" {
    fn GetCursorPos(point: *mut Point) -> i32;
}

#[derive(Serialize, Clone, Copy)]
struct CursorPosition {
    x: i32,
    y: i32,
}

pub fn init(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last = (i32::MIN, i32::MIN);
        loop {
            let mut point = Point { x: 0, y: 0 };
            if unsafe { GetCursorPos(&mut point) } != 0 && (point.x, point.y) != last {
                last = (point.x, point.y);
                let _ = app.emit("cursor-move", CursorPosition { x: point.x, y: point.y });
            }
            std::thread::sleep(Duration::from_millis(40));
        }
    });
}
