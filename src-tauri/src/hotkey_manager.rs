// 全局热键管理器

use once_cell::sync::OnceCell;
use std::sync::Mutex;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

// 全局状态
static APP_HANDLE: OnceCell<tauri::AppHandle> = OnceCell::new();
static MAIN_WINDOW_HANDLE: OnceCell<tauri::WebviewWindow> = OnceCell::new();
static CURRENT_TOGGLE_SHORTCUT: Mutex<Option<String>> = Mutex::new(None);
static CURRENT_PREVIEW_SHORTCUT: Mutex<Option<String>> = Mutex::new(None);
static HOTKEYS_ENABLED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);


// 初始化热键管理器
pub fn initialize_hotkey_manager(app_handle: tauri::AppHandle, window: tauri::WebviewWindow) {
    APP_HANDLE.set(app_handle).ok();
    MAIN_WINDOW_HANDLE.set(window).ok();
}

// 注册主窗口切换快捷键
pub fn register_toggle_hotkey(shortcut_str: &str) -> Result<(), String> {
    let app_handle = APP_HANDLE.get().ok_or("热键管理器未初始化")?;

    unregister_toggle_hotkey();

    let shortcut = parse_shortcut(shortcut_str)
        .map_err(|e| format!("解析快捷键失败: {}", e))?;

    app_handle
        .global_shortcut()
        .on_shortcut(shortcut.clone(), move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                handle_toggle_hotkey(app);
            }
        })
        .map_err(|e| format!("注册快捷键失败: {}", e))?;

    *CURRENT_TOGGLE_SHORTCUT.lock().unwrap() = Some(shortcut_str.to_string());
    
    println!("已注册主窗口切换快捷键: {}", shortcut_str);
    Ok(())
}

// 注册预览窗口快捷键
pub fn register_preview_hotkey(shortcut_str: &str) -> Result<(), String> {
    let app_handle = APP_HANDLE.get().ok_or("热键管理器未初始化")?;
    
    unregister_preview_hotkey();

    let shortcut = parse_shortcut(shortcut_str)
        .map_err(|e| format!("解析快捷键失败: {}", e))?;

    app_handle
        .global_shortcut()
        .on_shortcut(shortcut.clone(), move |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                handle_preview_hotkey_pressed(app);
            } else if event.state == ShortcutState::Released {
                handle_preview_hotkey_released(app);
            }
        })
        .map_err(|e| format!("注册预览快捷键失败: {}", e))?;

    *CURRENT_PREVIEW_SHORTCUT.lock().unwrap() = Some(shortcut_str.to_string());
    
    println!("已注册预览窗口快捷键: {}", shortcut_str);
    Ok(())
}

// 注销主窗口快捷键
pub fn unregister_toggle_hotkey() {
    if let Some(app_handle) = APP_HANDLE.get() {
        if let Some(shortcut_str) = CURRENT_TOGGLE_SHORTCUT.lock().unwrap().take() {

            if let Ok(shortcut) = parse_shortcut(&shortcut_str) {
                let _ = app_handle.global_shortcut().unregister(shortcut);
                println!("已注销主窗口切换快捷键: {}", shortcut_str);
            }
        }
    }
}

// 注销预览窗口快捷键
pub fn unregister_preview_hotkey() {
    if let Some(app_handle) = APP_HANDLE.get() {
        if let Some(shortcut_str) = CURRENT_PREVIEW_SHORTCUT.lock().unwrap().take() {
            if let Ok(shortcut) = parse_shortcut(&shortcut_str) {
                let _ = app_handle.global_shortcut().unregister(shortcut);
                println!("已注销预览窗口快捷键: {}", shortcut_str);
            }
        }
    }
}

// 注销所有快捷键
pub fn unregister_all_hotkeys() {
    unregister_toggle_hotkey();
    unregister_preview_hotkey();
}

// 更新主窗口切换快捷键
pub fn update_toggle_hotkey(shortcut_str: &str) -> Result<(), String> {
    register_toggle_hotkey(shortcut_str)
}

// 更新预览窗口快捷键
pub fn update_preview_hotkey(shortcut_str: &str) -> Result<(), String> {
    register_preview_hotkey(shortcut_str)
}

// 启用所有热键
pub fn enable_hotkeys() -> Result<(), String> {
    if HOTKEYS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        return Ok(());
    }

    if let Some(toggle_shortcut) = CURRENT_TOGGLE_SHORTCUT.lock().unwrap().clone() {
        register_toggle_hotkey(&toggle_shortcut)?;
    }
    
    if let Some(preview_shortcut) = CURRENT_PREVIEW_SHORTCUT.lock().unwrap().clone() {
        register_preview_hotkey(&preview_shortcut)?;
    }
    
    HOTKEYS_ENABLED.store(true, std::sync::atomic::Ordering::Relaxed);
    println!("已启用全局热键");
    Ok(())
}

// 禁用所有热键
pub fn disable_hotkeys() {
    if !HOTKEYS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }

    unregister_all_hotkeys();
    HOTKEYS_ENABLED.store(false, std::sync::atomic::Ordering::Relaxed);
    println!("已禁用全局热键");
}

// 检查热键是否启用
pub fn is_hotkeys_enabled() -> bool {
    HOTKEYS_ENABLED.load(std::sync::atomic::Ordering::Relaxed)
}


// 处理主窗口切换热键
fn handle_toggle_hotkey(_app: &tauri::AppHandle) {
    let settings = crate::settings::get_global_settings();
    if settings.app_filter_enabled {
        #[cfg(windows)]
        if !crate::app_filter::is_current_app_allowed() {
            return;
        }
    }

    if let Some(window) = MAIN_WINDOW_HANDLE.get() {
        let window_clone = window.clone();
        std::thread::spawn(move || {
            crate::window_management::toggle_webview_window_visibility(window_clone);
        });
    }
}

// 处理预览窗口热键按下
fn handle_preview_hotkey_pressed(app: &tauri::AppHandle) {
    let settings = crate::settings::get_global_settings();
   
    if !settings.preview_enabled {
        return;
    }

    if settings.app_filter_enabled {
        #[cfg(windows)]
        if !crate::app_filter::is_current_app_allowed() {
            return;
        }
    }

    let app_handle = app.clone();
    std::thread::spawn(move || {
        let _ = tauri::async_runtime::block_on(
            crate::preview_window::show_preview_window(app_handle),
        );
    });
}

// 处理预览窗口热键释放
fn handle_preview_hotkey_released(_app: &tauri::AppHandle) {
    let settings = crate::settings::get_global_settings();
    
    if !settings.preview_enabled {
        return;
    }

    let user_cancelled = crate::global_state::PREVIEW_CANCELLED_BY_USER
        .load(std::sync::atomic::Ordering::SeqCst);

    if user_cancelled {
        crate::global_state::PREVIEW_CANCELLED_BY_USER
            .store(false, std::sync::atomic::Ordering::SeqCst);
        std::thread::spawn(move || {
            let _ = tauri::async_runtime::block_on(
                crate::preview_window::hide_preview_window(),
            );
        });
    } else {
        std::thread::spawn(move || {
            let _ = tauri::async_runtime::block_on(
                crate::preview_window::paste_current_preview_item(),
            );
        });
    }
}

fn parse_shortcut(shortcut_str: &str) -> Result<Shortcut, String> {
    
    let normalized = shortcut_str
        .replace("Win+", "Super+")
        .replace("Ctrl+", "Control+");
    
    normalized.parse::<Shortcut>()
        .map_err(|e| format!("无效的快捷键格式: {}", e))
}

// 检查快捷键是否是Win+V
fn is_win_v_shortcut(shortcut_str: &str) -> bool {
    let normalized = shortcut_str.to_uppercase().replace(" ", "");
    normalized == "WIN+V" || normalized == "SUPER+V"
}
