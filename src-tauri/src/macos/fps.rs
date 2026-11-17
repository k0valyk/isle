use serde::Serialize;
use tauri::AppHandle;

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct FpsInfo {
    pub fps: Option<f32>,
    pub process: String,
    pub error: String,
}

pub fn init(_app: AppHandle) {}
