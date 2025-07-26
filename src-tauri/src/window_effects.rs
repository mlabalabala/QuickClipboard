use tauri::WebviewWindow;

#[cfg(target_os = "windows")]
use windows::Win32::{
    Foundation::HWND,
    Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE},
};

// Windows 11 圆角类型
#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum WindowCornerPreference {
    Default = 0,
    DoNotRound = 1,
    Round = 2,
    RoundSmall = 3,
}

// 设置窗口模糊效果
#[cfg(target_os = "windows")]
pub fn set_window_blur(window: &WebviewWindow) {
    use window_vibrancy::apply_acrylic;
    if let Err(e) = apply_acrylic(window, Some((255, 255, 255, 10))) {
        println!("设置窗口模糊效果失败: {}", e);
    }
}

// 设置窗口圆角
#[cfg(target_os = "windows")]
pub fn set_window_corner_radius(
    window: &WebviewWindow,
    corner_type: WindowCornerPreference,
) -> Result<(), String> {
    // 使用 Tauri 提供的直接方法获取 Windows 句柄
    if let Ok(hwnd_raw) = window.hwnd() {
        let hwnd = HWND(hwnd_raw.0 as isize);

        unsafe {
            let preference = corner_type as u32;
            let result = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &preference as *const u32 as *const std::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            );

            if result.is_ok() {
                // println!("窗口圆角设置成功: {:?}", corner_type);
                Ok(())
            } else {
                let error_msg = format!("设置窗口圆角失败: {:?}", result);
                println!("{}", error_msg);
                Err(error_msg)
            }
        }
    } else {
        let error_msg = "获取窗口句柄失败".to_string();
        println!("{}", error_msg);
        Err(error_msg)
    }
}

// 设置窗口为圆角（使用默认圆角大小）
#[cfg(target_os = "windows")]
pub fn set_window_rounded(window: &WebviewWindow) -> Result<(), String> {
    set_window_corner_radius(window, WindowCornerPreference::Round)
}

// 设置窗口为小圆角
#[cfg(target_os = "windows")]
pub fn set_window_rounded_small(window: &WebviewWindow) -> Result<(), String> {
    set_window_corner_radius(window, WindowCornerPreference::RoundSmall)
}

// 移除窗口圆角
#[cfg(target_os = "windows")]
pub fn remove_window_rounded(window: &WebviewWindow) -> Result<(), String> {
    set_window_corner_radius(window, WindowCornerPreference::DoNotRound)
}

// 非 Windows 平台的空实现
#[cfg(not(target_os = "windows"))]
pub fn set_window_blur(_window: &WebviewWindow) {
    println!("窗口模糊效果仅在 Windows 平台支持");
}

#[cfg(not(target_os = "windows"))]
pub fn set_window_rounded(_window: &WebviewWindow) -> Result<(), String> {
    println!("窗口圆角效果仅在 Windows 11 平台支持");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn set_window_rounded_small(_window: &WebviewWindow) -> Result<(), String> {
    println!("窗口圆角效果仅在 Windows 11 平台支持");
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn remove_window_rounded(_window: &WebviewWindow) -> Result<(), String> {
    println!("窗口圆角效果仅在 Windows 11 平台支持");
    Ok(())
}
