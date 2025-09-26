use tauri::Manager;
use std::sync::atomic::{AtomicBool, Ordering};

/// 截屏窗口状态管理
static SCREENSHOT_WINDOW_VISIBLE: AtomicBool = AtomicBool::new(false);

/// 截屏窗口管理器
pub struct ScreenshotWindowManager;

impl ScreenshotWindowManager {
    /// 显示截屏窗口
    pub fn show_screenshot_window(app: &tauri::AppHandle) -> Result<(), String> {
        // 获取截屏窗口
        let screenshot_window = app.get_webview_window("screenshot")
            .ok_or_else(|| "截屏窗口未找到".to_string())?;

        // 获取屏幕尺寸并设置窗口为全屏
        if let Err(_) = Self::set_fullscreen_size(app, &screenshot_window) {
            // 继续执行，使用默认尺寸
        }

        // 显示窗口
        screenshot_window.show().map_err(|e| format!("显示截屏窗口失败: {}", e))?;
        
        // 将窗口置顶并获得焦点
        screenshot_window.set_focus().map_err(|e| format!("设置截屏窗口焦点失败: {}", e))?;
        
        // 更新窗口可见状态
        SCREENSHOT_WINDOW_VISIBLE.store(true, Ordering::Relaxed);

        Ok(())
    }

    /// 隐藏截屏窗口
    pub fn hide_screenshot_window(app: &tauri::AppHandle) -> Result<(), String> {
        // 获取截屏窗口
        let screenshot_window = app.get_webview_window("screenshot")
            .ok_or_else(|| "截屏窗口未找到".to_string())?;

        // 隐藏窗口
        screenshot_window.hide().map_err(|e| format!("隐藏截屏窗口失败: {}", e))?;
        
        // 更新窗口可见状态
        SCREENSHOT_WINDOW_VISIBLE.store(false, Ordering::Relaxed);

        Ok(())
    }

    /// 切换截屏窗口显示状态
    pub fn toggle_screenshot_window(app: &tauri::AppHandle) -> Result<(), String> {
        if SCREENSHOT_WINDOW_VISIBLE.load(Ordering::Relaxed) {
            Self::hide_screenshot_window(app)
        } else {
            Self::show_screenshot_window(app)
        }
    }

    /// 检查截屏窗口是否可见
    pub fn is_screenshot_window_visible() -> bool {
        SCREENSHOT_WINDOW_VISIBLE.load(Ordering::Relaxed)
    }

    /// 设置窗口为跨所有显示器的全屏尺寸
    fn set_fullscreen_size(_app: &tauri::AppHandle, window: &tauri::WebviewWindow) -> Result<(), String> {
        use tauri::LogicalPosition;
        use tauri::LogicalSize;

        // 获取缩放因子并使用统一工具函数转换
        let scale_factor = window.scale_factor().unwrap_or(1.0);
        let (logical_x, logical_y, logical_width, logical_height) = 
            crate::screen_utils::ScreenUtils::get_css_virtual_screen_size(scale_factor)?;
        
        // 使用LogicalSize设置逻辑尺寸，让Tauri处理缩放
        let size = LogicalSize::new(logical_width, logical_height);
        window.set_size(size).map_err(|e| format!("设置窗口尺寸失败: {}", e))?;

        // 使用LogicalPosition设置逻辑位置
        let position = LogicalPosition::new(logical_x, logical_y);
        window.set_position(position).map_err(|e| format!("设置窗口位置失败: {}", e))?;

        
        Ok(())
    }

    /// 获取整个虚拟屏幕尺寸和位置（复用screen_utils）
    fn get_virtual_screen_info() -> Result<(i32, i32, u32, u32), String> {
        let (x, y, w, h) = crate::screen_utils::ScreenUtils::get_virtual_screen_size()?;
        Ok((x, y, w as u32, h as u32))
    }


    /// 获取所有显示器信息（物理像素格式，供后端使用）
    pub fn get_all_monitors() -> Result<Vec<crate::screen_utils::MonitorInfo>, String> {
        crate::screen_utils::ScreenUtils::get_all_monitors()
    }

    /// 初始化截屏窗口
    pub fn init_screenshot_window(app: &tauri::AppHandle) -> Result<(), String> {
        // 获取截屏窗口
        let screenshot_window = app.get_webview_window("screenshot")
            .ok_or_else(|| "截屏窗口未找到".to_string())?;

        // 确保窗口初始状态为隐藏
        let _ = screenshot_window.hide();
        SCREENSHOT_WINDOW_VISIBLE.store(false, Ordering::Relaxed);

        // 设置窗口关闭事件处理 - 隐藏而不是关闭
        let screenshot_window_clone = screenshot_window.clone();
        screenshot_window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 阻止默认的关闭行为
                api.prevent_close();
                // 隐藏窗口
                let _ = screenshot_window_clone.hide();
                SCREENSHOT_WINDOW_VISIBLE.store(false, Ordering::Relaxed);
            }
        });

        Ok(())
    }
}

/// 获取所有显示器信息（返回CSS像素格式，供前端使用）
#[tauri::command]
pub fn get_css_monitors(window: tauri::WebviewWindow) -> Result<Vec<crate::screen_utils::CssMonitorInfo>, String> {
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    crate::screen_utils::ScreenUtils::get_css_monitors(scale_factor)
}

/// 约束选区位置到合适的显示器边界内（复用贴边隐藏的边界逻辑）
#[tauri::command]
pub fn constrain_selection_bounds(
    window: tauri::WebviewWindow,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(f64, f64), String> {
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    
    // 转换为物理像素
    let physical_x = (x * scale_factor) as i32;
    let physical_y = (y * scale_factor) as i32;
    let physical_width = (width * scale_factor) as i32;
    let physical_height = (height * scale_factor) as i32;
    
    // 使用物理像素边界约束（复用窗口拖拽逻辑）
    let (constrained_physical_x, constrained_physical_y) = 
        crate::screen_utils::ScreenUtils::constrain_to_physical_bounds(
            physical_x, physical_y, physical_width, physical_height, &window
        )?;
    
    // 转换回CSS像素
    let constrained_x = constrained_physical_x as f64 / scale_factor;
    let constrained_y = constrained_physical_y as f64 / scale_factor;
    
    Ok((constrained_x, constrained_y))
}

/// 命令函数：显示截屏窗口
#[tauri::command]
pub fn show_screenshot_window(app: tauri::AppHandle) -> Result<(), String> {
    ScreenshotWindowManager::show_screenshot_window(&app)
}

/// 命令函数：隐藏截屏窗口
#[tauri::command]
pub fn hide_screenshot_window(app: tauri::AppHandle) -> Result<(), String> {
    ScreenshotWindowManager::hide_screenshot_window(&app)
}

/// 命令函数：切换截屏窗口显示状态
#[tauri::command]
pub fn toggle_screenshot_window(app: tauri::AppHandle) -> Result<(), String> {
    ScreenshotWindowManager::toggle_screenshot_window(&app)
}

/// 命令函数：检查截屏窗口是否可见
#[tauri::command]
pub fn is_screenshot_window_visible() -> bool {
    ScreenshotWindowManager::is_screenshot_window_visible()
}

/// 获取所有显示器信息的命令
#[tauri::command]
pub fn get_all_monitors() -> Result<Vec<crate::screen_utils::MonitorInfo>, String> {
    ScreenshotWindowManager::get_all_monitors()
}
