// 更新窗口管理
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use super::UpdateInfo;

// 显示更新窗口
pub async fn show_updater_window(
    app: AppHandle,
    update_info: UpdateInfo,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("updater") {
        window
            .set_focus()
            .map_err(|e| format!("无法聚焦更新窗口: {}", e))?;
        window
            .show()
            .map_err(|e| format!("无法显示更新窗口: {}", e))?;
        return Ok(());
    }

    super::set_update_info(update_info);

    let window = WebviewWindowBuilder::new(
        &app,
        "updater",
        WebviewUrl::App("updater/updater.html".into()),
    )
    .title("软件更新")
    .inner_size(480.0, 560.0)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .center()
    .always_on_top(true)
    .focused(true)
    .visible(false)
    .decorations(false)
    .skip_taskbar(true)
    .build()
    .map_err(|e| format!("创建更新窗口失败: {}", e))?;

    window
        .show()
        .map_err(|e| format!("显示更新窗口失败: {}", e))?;

    Ok(())
}

// 关闭更新窗口
pub fn close_updater_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("updater") {
        window
            .close()
            .map_err(|e| format!("关闭更新窗口失败: {}", e))?;
    }

    super::clear_update_info();
    
    Ok(())
}

