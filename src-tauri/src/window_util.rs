#[cfg(windows)]
mod native {
    #[link(name = "user32")]
    extern "system" {
        fn GetWindowLongPtrW(hwnd: isize, index: i32) -> isize;
        fn SetWindowLongPtrW(hwnd: isize, index: i32, value: isize) -> isize;
        fn SetWindowRgn(hwnd: isize, region: isize, redraw: i32) -> i32;
        fn SetWindowPos(
            hwnd: isize,
            insert_after: isize,
            x: i32,
            y: i32,
            cx: i32,
            cy: i32,
            flags: u32,
        ) -> i32;
        fn GetWindowRect(hwnd: isize, rect: *mut Rect) -> i32;
        fn GetSystemMetrics(index: i32) -> i32;
        fn GetCursorPos(point: *mut Point) -> i32;
        fn GetAsyncKeyState(key: i32) -> i16;
        fn MonitorFromPoint(point: Point, flags: u32) -> isize;
        fn MonitorFromWindow(hwnd: isize, flags: u32) -> isize;
        fn GetMonitorInfoW(monitor: isize, info: *mut MonitorInfo) -> i32;
    }

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    pub struct Point {
        pub x: i32,
        pub y: i32,
    }

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    pub struct Rect {
        pub left: i32,
        pub top: i32,
        pub right: i32,
        pub bottom: i32,
    }

    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    struct MonitorInfo {
        cb_size: u32,
        rc_monitor: Rect,
        rc_work: Rect,
        flags: u32,
    }

    #[link(name = "gdi32")]
    extern "system" {
        fn CreateRoundRectRgn(
            left: i32,
            top: i32,
            right: i32,
            bottom: i32,
            width: i32,
            height: i32,
        ) -> isize;
    }

    #[link(name = "shell32")]
    extern "system" {
        fn SHQueryUserNotificationState(state: *mut i32) -> i32;
    }

    const GWL_EXSTYLE: i32 = -20;
    const WS_EX_TOOLWINDOW: isize = 0x0000_0080;
    const WS_EX_APPWINDOW: isize = 0x0004_0000;
    const SWP_NOSIZE: u32 = 0x0001;
    const SWP_NOMOVE: u32 = 0x0002;
    const SWP_NOZORDER: u32 = 0x0004;
    const SWP_NOACTIVATE: u32 = 0x0010;
    const SWP_FRAMECHANGED: u32 = 0x0020;
    const SM_XVIRTUALSCREEN: i32 = 76;
    const SM_YVIRTUALSCREEN: i32 = 77;
    const SM_CXVIRTUALSCREEN: i32 = 78;
    const SM_CYVIRTUALSCREEN: i32 = 79;
    const MONITOR_DEFAULTTONEAREST: u32 = 2;
    const VK_LBUTTON: i32 = 0x01;

    pub fn cursor_position() -> Option<(i32, i32)> {
        let mut point = Point::default();
        if unsafe { GetCursorPos(&mut point) } == 0 {
            None
        } else {
            Some((point.x, point.y))
        }
    }

    pub fn left_button_down() -> bool {
        (unsafe { GetAsyncKeyState(VK_LBUTTON) } as u16 & 0x8000) != 0
    }

    pub fn work_area_at(x: i32, y: i32) -> Rect {
        unsafe {
            let monitor = MonitorFromPoint(Point { x, y }, MONITOR_DEFAULTTONEAREST);
            let mut info = MonitorInfo {
                cb_size: std::mem::size_of::<MonitorInfo>() as u32,
                ..MonitorInfo::default()
            };
            if monitor != 0 && GetMonitorInfoW(monitor, &mut info) != 0 {
                info.rc_work
            } else {
                let (vx, vy, vw, vh) = (
                    GetSystemMetrics(SM_XVIRTUALSCREEN),
                    GetSystemMetrics(SM_YVIRTUALSCREEN),
                    GetSystemMetrics(SM_CXVIRTUALSCREEN),
                    GetSystemMetrics(SM_CYVIRTUALSCREEN),
                );
                Rect {
                    left: vx,
                    top: vy,
                    right: vx + vw,
                    bottom: vy + vh,
                }
            }
        }
    }

    pub fn work_area_for_window(hwnd: isize) -> Rect {
        unsafe {
            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            let mut info = MonitorInfo {
                cb_size: std::mem::size_of::<MonitorInfo>() as u32,
                ..MonitorInfo::default()
            };
            if monitor != 0 && GetMonitorInfoW(monitor, &mut info) != 0 {
                info.rc_work
            } else {
                work_area_at(0, 0)
            }
        }
    }

    pub fn window_size(hwnd: isize) -> (i32, i32) {
        let mut rect = Rect::default();
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            (600, 400)
        } else {
            (
                (rect.right - rect.left).max(1),
                (rect.bottom - rect.top).max(1),
            )
        }
    }

    pub fn mark_tool_window(hwnd: isize) {
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            let updated = (style | WS_EX_TOOLWINDOW) & !WS_EX_APPWINDOW;
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, updated);
            SetWindowPos(
                hwnd,
                0,
                0,
                0,
                0,
                0,
                SWP_NOSIZE | SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );
        }
    }

    pub fn physical_position(hwnd: isize) -> Option<(i32, i32)> {
        let mut rect = Rect::default();
        let ok = unsafe { GetWindowRect(hwnd, &mut rect) };
        if ok == 0 {
            None
        } else {
            Some((rect.left, rect.top))
        }
    }

    pub fn place_physical(hwnd: isize, x: i32, y: i32) {
        unsafe {
            SetWindowPos(
                hwnd,
                0,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
    }

    pub fn clamp_physical(hwnd: isize, x: i32, y: i32) -> (i32, i32) {
        let mut rect = Rect::default();
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            return (x, y);
        }
        let width = (rect.right - rect.left).max(1);
        let height = (rect.bottom - rect.top).max(1);
        let (vx, vy, vw, vh) = unsafe {
            (
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        };
        if vw <= 0 || vh <= 0 {
            return (x, y);
        }
        (
            x.clamp(vx, (vx + vw - width).max(vx)),
            y.clamp(vy, (vy + vh - height).max(vy)),
        )
    }

    pub fn place_docked(hwnd: isize, dock: &str) {
        let work = work_area_for_window(hwnd);
        let (ww, wh) = window_size(hwnd);
        let (current_x, current_y) =
            physical_position(hwnd).unwrap_or((work.left, work.top));
        let (x, y) = match dock {
            "left" => (
                work.left,
                current_y.clamp(work.top, (work.bottom - wh).max(work.top)),
            ),
            "right" => (
                (work.right - ww).max(work.left),
                current_y.clamp(work.top, (work.bottom - wh).max(work.top)),
            ),
            "bottom" => (
                work.left + ((work.right - work.left - ww) / 2).max(0),
                (work.bottom - wh).max(work.top),
            ),
            "free" => (current_x, current_y),
            _ => (
                work.left + ((work.right - work.left - ww) / 2).max(0),
                work.top,
            ),
        };
        place_physical(hwnd, x, y);
    }

    pub fn apply_region(hwnd: isize, x: i32, y: i32, w: i32, h: i32, radius: f64) {
        if w <= 0 || h <= 0 {
            return;
        }
        unsafe {
            let diameter = (radius * 2.0).round().clamp(4.0, 4096.0) as i32;
            let region = CreateRoundRectRgn(x, y, x + w + 1, y + h + 1, diameter, diameter);
            if region != 0 {
                SetWindowRgn(hwnd, region, 0);
            }
        }
    }

    pub fn fullscreen_active() -> bool {
        let mut state: i32 = 0;
        let result = unsafe { SHQueryUserNotificationState(&mut state) };
        if result != 0 {
            return false;
        }
        // QUNS_BUSY = 2, QUNS_RUNNING_D3D_FULL_SCREEN = 3, QUNS_PRESENTATION_MODE = 4
        state == 2 || state == 3 || state == 4
    }
}

#[cfg(windows)]
fn hwnd_of(window: &tauri::WebviewWindow) -> Option<isize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = window.window_handle().ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()),
        _ => None,
    }
}

pub fn mark_tool_window(window: &tauri::WebviewWindow) {
    #[cfg(windows)]
    if let Some(hwnd) = hwnd_of(window) {
        native::mark_tool_window(hwnd);
    }
    #[cfg(not(windows))]
    let _ = window;
}

pub fn apply_region(window: &tauri::WebviewWindow, x: i32, y: i32, w: i32, h: i32, radius: f64) {
    #[cfg(windows)]
    if let Some(hwnd) = hwnd_of(window) {
        native::apply_region(hwnd, x, y, w, h, radius);
    }
    #[cfg(not(windows))]
    let _ = (window, x, y, w, h, radius);
}

pub fn fullscreen_active() -> bool {
    #[cfg(windows)]
    {
        native::fullscreen_active()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[derive(Clone, Copy, Debug, Default, serde::Deserialize)]
pub struct IslandLayout {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

pub fn cursor_position() -> Option<(i32, i32)> {
    #[cfg(windows)]
    {
        native::cursor_position()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

pub fn left_button_down() -> bool {
    #[cfg(windows)]
    {
        native::left_button_down()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub fn work_area_at(x: i32, y: i32) -> (i32, i32, i32, i32) {
    #[cfg(windows)]
    {
        let rect = native::work_area_at(x, y);
        (rect.left, rect.top, rect.right, rect.bottom)
    }
    #[cfg(not(windows))]
    {
        let _ = (x, y);
        (0, 0, 1920, 1080)
    }
}

pub fn work_area_for(window: &tauri::WebviewWindow) -> (i32, i32, i32, i32) {
    #[cfg(windows)]
    if let Some(hwnd) = hwnd_of(window) {
        let rect = native::work_area_for_window(hwnd);
        return (rect.left, rect.top, rect.right, rect.bottom);
    }
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());
    match monitor {
        Some(monitor) => {
            let position = monitor.position();
            let size = monitor.size();
            (
                position.x,
                position.y,
                position.x + size.width as i32,
                position.y + size.height as i32,
            )
        }
        None => (0, 0, 1920, 1080),
    }
}

pub fn window_size(window: &tauri::WebviewWindow) -> (i32, i32) {
    #[cfg(windows)]
    if let Some(hwnd) = hwnd_of(window) {
        return native::window_size(hwnd);
    }
    window
        .outer_size()
        .map(|size| (size.width as i32, size.height as i32))
        .unwrap_or((600, 400))
}

pub fn place_docked(window: &tauri::WebviewWindow, dock: &str) {
    #[cfg(windows)]
    if let Some(hwnd) = hwnd_of(window) {
        native::place_docked(hwnd, dock);
        return;
    }
    // Cross-platform fallback (macOS): mirror the native maths with Tauri's
    // monitor information instead of Win32 work-area queries.
    let (left, top, right, bottom) = work_area_for(window);
    let (width, height) = window_size(window);
    let (current_x, current_y) = physical_position(window).unwrap_or((left, top));
    let (x, y) = match dock {
        "left" => (left, current_y.clamp(top, (bottom - height).max(top))),
        "right" => (
            (right - width).max(left),
            current_y.clamp(top, (bottom - height).max(top)),
        ),
        "bottom" => (
            left + ((right - left - width) / 2).max(0),
            (bottom - height).max(top),
        ),
        "free" => (current_x, current_y),
        _ => (left + ((right - left - width) / 2).max(0), top),
    };
    place_at(window, x, y);
}

pub fn place_top_center(window: &tauri::WebviewWindow) {
    place_docked(window, "top");
}

/// Recentre the island on the primary display. Used by "reset position" so a
/// window dragged (or stranded) on a secondary monitor can always come back.
pub fn place_primary_top_center(window: &tauri::WebviewWindow) {
    let Some(monitor) = window.primary_monitor().ok().flatten() else {
        place_top_center(window);
        return;
    };
    let position = monitor.position();
    let width = window
        .outer_size()
        .map(|size| size.width as i32)
        .unwrap_or(600);

    #[cfg(windows)]
    let (left, top, right) = {
        let (left, top, right, _bottom) = work_area_at(position.x + 1, position.y + 1);
        (left, top, right)
    };
    #[cfg(not(windows))]
    let (left, top, right) = {
        // macOS keeps the menu bar outside the usable area.
        let menu = if cfg!(target_os = "macos") { 28 } else { 0 };
        (
            position.x,
            position.y + menu,
            position.x + monitor.size().width as i32,
        )
    };

    let x = left + ((right - left - width) / 2).max(0);
    place_at(window, x, top);
    // Crossing monitors changes the window's physical size when the DPI
    // differs, so recentre once after the move settles.
    std::thread::sleep(std::time::Duration::from_millis(90));
    if let Ok(size) = window.outer_size() {
        let width = size.width as i32;
        let x = left + ((right - left - width) / 2).max(0);
        place_at(window, x, top);
    }
}

/// Reposition the window so the island stays where it was after its alignment
/// inside the 600×400 host window changes (for example left dock → free).
pub fn place_for_layout(
    window: &tauri::WebviewWindow,
    layout: IslandLayout,
    island_left: i32,
    island_top: i32,
    dock: &str,
) {
    let (window_width, window_height) = window_size(window);
    let work = work_area_for(window);
    let x = match dock {
        "left" => work.0,
        "right" => (work.2 - window_width).max(work.0),
        "top" | "bottom" => work.0 + ((work.2 - work.0 - window_width) / 2).max(0),
        _ => island_left - ((window_width - layout.width) / 2),
    };
    let y = match dock {
        "bottom" => (work.3 - window_height).max(work.1),
        "top" => work.1,
        "left" | "right" => island_top.clamp(work.1, (work.3 - window_height).max(work.1)),
        _ => island_top,
    };
    place_at(window, x, y);
}

pub fn collapsed_region(
    window: &tauri::WebviewWindow,
    character: bool,
    battery: bool,
    dock: &str,
) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let (width, height) = window_size(window);
    let collapsed = (142.0 + if character { 32.0 } else { 0.0 } + if battery { 48.0 } else { 0.0 })
        * scale;
    let x = match dock {
        "left" => 0,
        "right" => (width - collapsed as i32).max(0),
        _ => ((width as f64 - collapsed) / 2.0).max(0.0) as i32,
    };
    let y = if dock == "bottom" {
        (height - (34.0 * scale) as i32).max(0)
    } else {
        0
    };
    apply_region(
        window,
        x,
        y,
        collapsed as i32,
        (34.0 * scale) as i32,
        999.0,
    );
}

/// Clamp a window position to the physical virtual desktop so the island can be
/// dragged to any screen (including monitors with negative coordinates) but
/// never completely lost off-screen.
pub fn clamp_to_monitors(window: &tauri::WebviewWindow, x: i32, y: i32) -> (i32, i32) {
    #[cfg(windows)]
    if let Some(hwnd) = hwnd_of(window) {
        return native::clamp_physical(hwnd, x, y);
    }
    let monitors = window.available_monitors().unwrap_or_default();
    if monitors.is_empty() {
        return (x, y);
    }
    let mut left = i32::MAX;
    let mut top = i32::MAX;
    let mut right = i32::MIN;
    let mut bottom = i32::MIN;
    for monitor in monitors {
        let position = monitor.position();
        let size = monitor.size();
        left = left.min(position.x);
        top = top.min(position.y);
        right = right.max(position.x + size.width as i32);
        bottom = bottom.max(position.y + size.height as i32);
    }
    let window_size = window
        .outer_size()
        .unwrap_or(tauri::PhysicalSize::new(600, 400));
    let max_x = (right - window_size.width as i32).max(left);
    let max_y = (bottom - window_size.height as i32).max(top);
    (x.clamp(left, max_x), y.clamp(top, max_y))
}

pub fn place_at(window: &tauri::WebviewWindow, x: i32, y: i32) {
    let (x, y) = clamp_to_monitors(window, x, y);
    #[cfg(windows)]
    if let Some(hwnd) = hwnd_of(window) {
        native::place_physical(hwnd, x, y);
        return;
    }
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
}

pub fn physical_position(window: &tauri::WebviewWindow) -> Option<(i32, i32)> {
    #[cfg(windows)]
    if let Some(hwnd) = hwnd_of(window) {
        return native::physical_position(hwnd);
    }
    window
        .outer_position()
        .ok()
        .map(|position| (position.x, position.y))
}
