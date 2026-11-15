use serde::Serialize;

#[repr(C)]
struct SystemPowerStatus {
    ac_line_status: u8,
    battery_flag: u8,
    battery_life_percent: u8,
    system_status_flag: u8,
    battery_life_time: u32,
    battery_full_life_time: u32,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetSystemPowerStatus(status: *mut SystemPowerStatus) -> i32;
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BatteryStatus {
    pub present: bool,
    pub percent: Option<u8>,
    pub charging: bool,
    pub ac: bool,
}

#[tauri::command]
pub fn battery_status() -> BatteryStatus {
    let mut status = SystemPowerStatus {
        ac_line_status: 255,
        battery_flag: 255,
        battery_life_percent: 255,
        system_status_flag: 0,
        battery_life_time: u32::MAX,
        battery_full_life_time: u32::MAX,
    };
    let ok = unsafe { GetSystemPowerStatus(&mut status) } != 0;
    if !ok {
        return BatteryStatus {
            present: false,
            percent: None,
            charging: false,
            ac: true,
        };
    }

    let present = status.battery_flag != 128 && status.battery_flag != 255;
    let percent = if present && status.battery_life_percent <= 100 {
        Some(status.battery_life_percent)
    } else {
        None
    };

    BatteryStatus {
        present,
        percent,
        charging: status.battery_flag & 8 != 0,
        ac: status.ac_line_status == 1,
    }
}
