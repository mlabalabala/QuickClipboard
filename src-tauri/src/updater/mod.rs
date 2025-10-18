// 更新器模块
pub mod window;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    #[serde(rename = "currentVersion")]
    pub current_version: String,
    pub date: Option<String>,
    pub body: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "htmlUrl")]
    pub html_url: Option<String>,
    pub platforms: Option<serde_json::Value>,
    #[serde(rename = "forceUpdate", default)]
    pub force_update: bool,
}

// 全局更新信息存储
static UPDATE_INFO: Lazy<Mutex<Option<UpdateInfo>>> = Lazy::new(|| Mutex::new(None));

// 设置更新信息
pub fn set_update_info(info: UpdateInfo) {
    if let Ok(mut data) = UPDATE_INFO.lock() {
        *data = Some(info);
    }
}

// 获取更新信息
pub fn get_update_info() -> Option<UpdateInfo> {
    UPDATE_INFO.lock().ok().and_then(|data| data.clone())
}

// 清除更新信息
pub fn clear_update_info() {
    if let Ok(mut data) = UPDATE_INFO.lock() {
        *data = None;
    }
}

