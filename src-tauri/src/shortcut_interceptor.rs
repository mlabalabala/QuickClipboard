// 快捷键拦截器 - 专门用于拦截主窗口快捷键，防止触发系统剪贴板

use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::Manager;

// 全局状态
#[cfg(windows)]
static SHORTCUT_HOOK_HANDLE: Mutex<Option<windows::Win32::UI::WindowsAndMessaging::HHOOK>> =
    Mutex::new(None);

#[cfg(windows)]
pub static SHORTCUT_INTERCEPTION_ENABLED: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
pub static MAIN_WINDOW_HANDLE: OnceCell<tauri::WebviewWindow> = OnceCell::new();


// 导航按键监听状态
#[cfg(windows)]
static NAVIGATION_KEYS_ENABLED: AtomicBool = AtomicBool::new(false);

// 翻译进行状态（用于暂时禁用导航按键）
#[cfg(windows)]
static TRANSLATION_IN_PROGRESS: AtomicBool = AtomicBool::new(false);


// =================== 辅助函数 ===================

// 检查是否匹配导航快捷键
#[cfg(windows)]
fn matches_navigation_shortcut(
    vk_code: u32,
    ctrl_pressed: bool,
    shift_pressed: bool,
    alt_pressed: bool,
    win_pressed: bool,
    shortcut_str: &str,
) -> bool {
    if let Some(parsed) = crate::global_state::parse_shortcut(shortcut_str) {
        // 检查修饰键是否匹配
        if ctrl_pressed != parsed.ctrl
            || shift_pressed != parsed.shift
            || alt_pressed != parsed.alt
            || win_pressed != parsed.win
        {
            return false;
        }
        
        // 检查主键是否匹配
        vk_code == parsed.key_code
    } else {
        false
    }
}

// =================== 键盘钩子函数 ===================

#[cfg(windows)]
unsafe extern "system" fn shortcut_hook_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, HC_ACTION, KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN,
        WM_SYSKEYUP,
    };

    if code == HC_ACTION as i32 && SHORTCUT_INTERCEPTION_ENABLED.load(Ordering::Relaxed) {
        let kbd_data = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let vk_code = kbd_data.vkCode;

        let ctrl_pressed = (GetAsyncKeyState(0x11) & 0x8000u16 as i16) != 0; // VK_CONTROL
        let shift_pressed = (GetAsyncKeyState(0x10) & 0x8000u16 as i16) != 0; // VK_SHIFT
        let alt_pressed = (GetAsyncKeyState(0x12) & 0x8000u16 as i16) != 0; // VK_MENU
        let win_pressed = (GetAsyncKeyState(0x5B) & 0x8000u16 as i16) != 0 // VK_LWIN
            || (GetAsyncKeyState(0x5C) & 0x8000u16 as i16) != 0; // VK_RWIN

        let settings = crate::settings::get_global_settings();
        // 检查应用黑白名单过滤
        if settings.app_filter_enabled {
            if !crate::app_filter::is_current_app_allowed() {
                return CallNextHookEx(None, code, wparam, lparam);
            }
        }

        let is_own_window = if let Some(window) = MAIN_WINDOW_HANDLE.get() {
            crate::window_management::is_current_window_own_app(window)
        } else {
            false
        };

        // 处理导航按键（仅当启用且窗口应该接收按键且翻译未进行时）
        if NAVIGATION_KEYS_ENABLED.load(Ordering::Relaxed)
            && !TRANSLATION_IN_PROGRESS.load(Ordering::Relaxed)
        {
            if let Some(window) = MAIN_WINDOW_HANDLE.get() {
                // 检查窗口是否应该接收导航按键
                if crate::window_management::should_receive_navigation_keys(window) {
                    // 获取当前设置
                    let settings = crate::settings::get_global_settings();
                    
                    if wparam.0 as u32 == WM_KEYDOWN {
                        // 检查各种导航快捷键并直接发送动作事件
                        if matches_navigation_shortcut(vk_code, ctrl_pressed, shift_pressed, alt_pressed, win_pressed, &settings.navigate_up_shortcut) {
                            emit_navigation_action("navigate-up");
                            return LRESULT(1);
                        }
                        else if matches_navigation_shortcut(vk_code, ctrl_pressed, shift_pressed, alt_pressed, win_pressed, &settings.navigate_down_shortcut) {
                            emit_navigation_action("navigate-down");
                            return LRESULT(1);
                        }
                        else if matches_navigation_shortcut(vk_code, ctrl_pressed, shift_pressed, alt_pressed, win_pressed, &settings.tab_left_shortcut) {
                            emit_navigation_action("tab-left");
                            return LRESULT(1);
                        }
                        else if matches_navigation_shortcut(vk_code, ctrl_pressed, shift_pressed, alt_pressed, win_pressed, &settings.tab_right_shortcut) {
                            emit_navigation_action("tab-right");
                            return LRESULT(1);
                        }
                        else if matches_navigation_shortcut(vk_code, ctrl_pressed, shift_pressed, alt_pressed, win_pressed, &settings.focus_search_shortcut) {
                            emit_navigation_action("focus-search");
                            return LRESULT(1);
                        }
                        else if matches_navigation_shortcut(vk_code, ctrl_pressed, shift_pressed, alt_pressed, win_pressed, &settings.hide_window_shortcut) {
                            emit_navigation_action("hide-window");
                            return LRESULT(1);
                        }
                        else if matches_navigation_shortcut(vk_code, ctrl_pressed, shift_pressed, alt_pressed, win_pressed, &settings.execute_item_shortcut) {
                            emit_navigation_action("execute-item");
                            return LRESULT(1);
                        }
                        else if matches_navigation_shortcut(vk_code, ctrl_pressed, shift_pressed, alt_pressed, win_pressed, &settings.previous_group_shortcut) {
                            emit_navigation_action("previous-group");
                            return LRESULT(1);
                        }
                        else if matches_navigation_shortcut(vk_code, ctrl_pressed, shift_pressed, alt_pressed, win_pressed, &settings.next_group_shortcut) {
                            emit_navigation_action("next-group");
                            return LRESULT(1);
                        }
                        else if matches_navigation_shortcut(vk_code, ctrl_pressed, shift_pressed, alt_pressed, win_pressed, &settings.toggle_pin_shortcut) {
                            emit_navigation_action("toggle-pin");
                            return LRESULT(1);
                        }
                    }
                }
            }
        }

        // 监听 Ctrl+V 触发粘贴音效（不拦截事件）
        if wparam.0 as u32 == WM_KEYDOWN {
            if vk_code == 0x56 && ctrl_pressed && !shift_pressed && !alt_pressed && !win_pressed {
                std::thread::spawn(|| {
                    crate::sound_manager::play_paste_sound();
                });
            }
        }
    }

    CallNextHookEx(None, code, wparam, lparam)
}

// 发送导航动作事件到前端
#[cfg(windows)]
fn emit_navigation_action(action: &str) {
    use tauri::Emitter;

    if let Some(window) = MAIN_WINDOW_HANDLE.get() {
        let _ = window.emit(
            "navigation-action",
            serde_json::json!({
                "action": action
            }),
        );
    }
}

// =================== 公共接口 ===================

// 初始化快捷键拦截器
#[cfg(windows)]
pub fn initialize_shortcut_interceptor(window: tauri::WebviewWindow) {
    MAIN_WINDOW_HANDLE.set(window).ok();
}

// 安装快捷键钩子
#[cfg(windows)]
pub fn install_shortcut_hook() {
    use windows::Win32::Foundation::HINSTANCE;
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowsHookExW, WH_KEYBOARD_LL};

    let mut hook_handle = SHORTCUT_HOOK_HANDLE.lock().unwrap();
    if hook_handle.is_none() {
        unsafe {
            match SetWindowsHookExW(WH_KEYBOARD_LL, Some(shortcut_hook_proc), HINSTANCE(0), 0) {
                Ok(hook) => {
                    *hook_handle = Some(hook);
                }
                Err(_e) => {
                    // 静默处理错误
                }
            }
        }
    }
}

// 启用快捷键拦截
#[cfg(windows)]
pub fn enable_shortcut_interception() {
    SHORTCUT_INTERCEPTION_ENABLED.store(true, Ordering::SeqCst);
}

// 禁用快捷键拦截
#[cfg(windows)]
pub fn disable_shortcut_interception() {
    SHORTCUT_INTERCEPTION_ENABLED.store(false, Ordering::SeqCst);
}

// 查询快捷键拦截是否启用
#[cfg(windows)]
pub fn is_interception_enabled() -> bool {
    SHORTCUT_INTERCEPTION_ENABLED.load(Ordering::SeqCst)
}


// 启用导航按键监听
#[cfg(windows)]
pub fn enable_navigation_keys() {
    NAVIGATION_KEYS_ENABLED.store(true, Ordering::SeqCst);
}

// 禁用导航按键监听
#[cfg(windows)]
pub fn disable_navigation_keys() {
    NAVIGATION_KEYS_ENABLED.store(false, Ordering::SeqCst);
}

// 设置翻译进行状态（禁用导航按键）
#[cfg(windows)]
pub fn set_translation_in_progress(in_progress: bool) {
    TRANSLATION_IN_PROGRESS.store(in_progress, Ordering::SeqCst);
}

// 非Windows平台的空实现
#[cfg(not(windows))]
pub fn initialize_shortcut_interceptor(_window: tauri::WebviewWindow) {}

#[cfg(not(windows))]
pub fn install_shortcut_hook() {}

#[cfg(not(windows))]
pub fn uninstall_shortcut_hook() {}

#[cfg(not(windows))]
pub fn enable_shortcut_interception() {}

#[cfg(not(windows))]
pub fn disable_shortcut_interception() {}

#[cfg(not(windows))]
pub fn is_interception_enabled() -> bool {
    false
}

#[cfg(not(windows))]
pub fn enable_navigation_keys() {}

#[cfg(not(windows))]
pub fn disable_navigation_keys() {}

#[cfg(not(windows))]
pub fn is_navigation_keys_enabled() -> bool {
    false
}

#[cfg(not(windows))]
pub fn set_translation_in_progress(_in_progress: bool) {}

#[cfg(not(windows))]
pub fn is_translation_in_progress() -> bool {
    false
}
