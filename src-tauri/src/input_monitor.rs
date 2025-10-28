use once_cell::sync::OnceCell;
use rdev::{listen, Event, EventType, Key};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::collections::HashSet;
use std::thread;
use std::time::Duration;
use tauri::{Emitter, WebviewWindow};

// 全局状态
pub static MAIN_WINDOW_HANDLE: OnceCell<WebviewWindow> = OnceCell::new();
static MONITORING_ACTIVE: AtomicBool = AtomicBool::new(false);
static MONITORING_THREAD_HANDLE: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);

// 导航键启用状态
static NAVIGATION_KEYS_ENABLED: AtomicBool = AtomicBool::new(false);

// 鼠标监听相关的全局状态
pub static MOUSE_MONITORING_ENABLED: AtomicBool = AtomicBool::new(false);

// 鼠标监听需求跟踪
static MOUSE_MONITORING_REQUESTS: std::sync::LazyLock<Mutex<HashSet<String>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashSet::new()));

// 按键状态跟踪
struct KeyboardState {
    ctrl: bool,
    alt: bool,
    shift: bool,
    meta: bool,
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        }
    }
}

static KEYBOARD_STATE: Mutex<KeyboardState> = Mutex::new(KeyboardState {
    ctrl: false,
    alt: false,
    shift: false,
    meta: false,
});

// 导航键焦点状态跟踪
static NAVIGATION_KEY_FOCUSED: AtomicBool = AtomicBool::new(false);

// 启动输入监控系统
pub fn start_input_monitoring() {
    if MONITORING_ACTIVE.load(Ordering::SeqCst) {
        return;
    }

    MONITORING_ACTIVE.store(true, Ordering::SeqCst);

    let monitoring_handle = std::thread::spawn(|| {
        if let Err(error) = listen(move |event| {
            if !MONITORING_ACTIVE.load(Ordering::SeqCst) {
                return;
            }
            
            handle_input_event(event);
        }) {
            eprintln!("输入监控错误: {:?}", error);
        }
    });

    if let Ok(mut handle) = MONITORING_THREAD_HANDLE.lock() {
        *handle = Some(monitoring_handle);
    }
}

// 停止输入监控系统
pub fn stop_input_monitoring() {
    MONITORING_ACTIVE.store(false, Ordering::SeqCst);
}

// 查询监控是否处于活动状态
pub fn is_monitoring_active() -> bool {
    MONITORING_ACTIVE.load(Ordering::SeqCst)
}

// 启用导航键监听
pub fn enable_navigation_keys() {
    NAVIGATION_KEYS_ENABLED.store(true, Ordering::SeqCst);
}

// 禁用导航键监听
pub fn disable_navigation_keys() {
    NAVIGATION_KEYS_ENABLED.store(false, Ordering::SeqCst);
}

// 查询导航键是否启用
pub fn is_navigation_keys_enabled() -> bool {
    NAVIGATION_KEYS_ENABLED.load(Ordering::SeqCst)
}

// 请求启用鼠标监听（带来源标识）
pub fn request_mouse_monitoring(source: &str) {
    if let Ok(mut requests) = MOUSE_MONITORING_REQUESTS.lock() {
        requests.insert(source.to_string());
    }
    MOUSE_MONITORING_ENABLED.store(true, Ordering::Relaxed);
}

// 释放鼠标监听请求（带来源标识）
pub fn release_mouse_monitoring(source: &str) {
    if let Ok(mut requests) = MOUSE_MONITORING_REQUESTS.lock() {
        requests.remove(source);
        if requests.is_empty() {
            MOUSE_MONITORING_ENABLED.store(false, Ordering::Relaxed);
        }
    }
}

// 启用鼠标监听
pub fn enable_mouse_monitoring() {
    request_mouse_monitoring("legacy");
}

// 禁用鼠标监听
pub fn disable_mouse_monitoring() {
    release_mouse_monitoring("legacy");
}

// 检查鼠标监听是否启用
pub fn is_mouse_monitoring_enabled() -> bool {
    MOUSE_MONITORING_ENABLED.load(Ordering::Relaxed)
}

// 获取当前键盘修饰键状态（供其他模块使用）
pub fn get_modifier_keys_state() -> (bool, bool, bool, bool) {
    if let Ok(state) = KEYBOARD_STATE.lock() {
        (state.ctrl, state.alt, state.shift, state.meta)
    } else {
        (false, false, false, false)
    }
}

// 处理输入事件
fn handle_input_event(event: Event) {
    match event.event_type {
        EventType::KeyPress(key) => handle_key_press(key),
        EventType::KeyRelease(key) => handle_key_release(key),
        EventType::ButtonPress(button) => handle_mouse_button_press(button),
        EventType::ButtonRelease(button) => handle_mouse_button_release(button),
        EventType::MouseMove { x, y } => handle_mouse_move(x, y),
        EventType::Wheel { delta_x, delta_y } => handle_mouse_wheel(delta_x, delta_y),
    }
}

// 处理按键按下
fn handle_key_press(key: Key) {
    // 更新修饰键状态
    if let Ok(mut state) = KEYBOARD_STATE.lock() {
        match key {
            Key::ControlLeft | Key::ControlRight => state.ctrl = true,
            Key::Alt | Key::AltGr => state.alt = true,
            Key::ShiftLeft | Key::ShiftRight => state.shift = true,
            Key::MetaLeft | Key::MetaRight => state.meta = true,
            _ => {}
        }
    }

    // 检查应用过滤
    let settings = crate::settings::get_global_settings();
    if settings.app_filter_enabled {
        #[cfg(windows)]
        if !crate::app_filter::is_current_app_allowed() {
            return;
        }
    }

    // 处理导航快捷键（按下时获得焦点）
    if NAVIGATION_KEYS_ENABLED.load(Ordering::SeqCst) {
        if let Some(need_focus) = handle_navigation_key_press(key) {
            // 导航键被触发，根据需要获得焦点
            if need_focus {
                acquire_focus_for_navigation();
            }
        }
    }

    handle_number_shortcut(key);

    handle_paste_sound(key);

    handle_translation_cancel(key);
}

// 处理按键释放
fn handle_key_release(key: Key) {
    if let Ok(mut state) = KEYBOARD_STATE.lock() {
        match key {
            Key::ControlLeft | Key::ControlRight => state.ctrl = false,
            Key::Alt | Key::AltGr => state.alt = false,
            Key::ShiftLeft | Key::ShiftRight => state.shift = false,
            Key::MetaLeft | Key::MetaRight => state.meta = false,
            _ => {}
        }
    }
}

// 处理导航键按下（返回是否触发了导航动作，以及是否需要获取焦点）
fn handle_navigation_key_press(key: Key) -> Option<bool> {
    if let Some(window) = MAIN_WINDOW_HANDLE.get() {
        if !crate::window_management::should_receive_navigation_keys(window) {
            return None;
        }
    } else {
        return None;
    }

    let settings = crate::settings::get_global_settings();
    
    // 检查翻译进行状态
    #[cfg(windows)]
    if crate::global_state::TRANSLATION_IN_PROGRESS.load(Ordering::SeqCst) {
        return None;
    }

    let shortcuts = [
        (&settings.navigate_up_shortcut, "navigate-up", true),
        (&settings.navigate_down_shortcut, "navigate-down", true),
        (&settings.tab_left_shortcut, "tab-left", true),
        (&settings.tab_right_shortcut, "tab-right", true),
        (&settings.focus_search_shortcut, "focus-search", false),
        (&settings.hide_window_shortcut, "hide-window", false),
        (&settings.execute_item_shortcut, "execute-item", false),
        (&settings.previous_group_shortcut, "previous-group", true),
        (&settings.next_group_shortcut, "next-group", true),
        (&settings.toggle_pin_shortcut, "toggle-pin", true),
    ];

    for (shortcut_str, action, need_focus) in shortcuts {
        if check_shortcut_match(key, shortcut_str) {
            emit_navigation_action(action);
            return Some(need_focus);
        }
    }
    
    None
}

// 为导航获得焦点
fn acquire_focus_for_navigation() {
    let was_focused = NAVIGATION_KEY_FOCUSED.swap(true, Ordering::SeqCst);
    
    if !was_focused {
        #[cfg(windows)]
        {
            use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
            unsafe {
                let hwnd = GetForegroundWindow();
                crate::window_management::set_last_focus_hwnd(hwnd.0);
            }
        }

        if let Some(window) = MAIN_WINDOW_HANDLE.get() {
            let _ = window.set_focus();
        }
    }

    thread::spawn(|| {
        thread::sleep(Duration::from_millis(100));
        restore_focus_from_navigation();
    });
}

// 从导航恢复焦点
fn restore_focus_from_navigation() {
    if !NAVIGATION_KEY_FOCUSED.load(Ordering::SeqCst) {
        return;
    }

    // 恢复之前保存的焦点
    let _ = crate::window_management::restore_last_focus();

    NAVIGATION_KEY_FOCUSED.store(false, Ordering::SeqCst);
}

// 检查当前按键是否匹配快捷键字符串
fn check_shortcut_match(key: Key, shortcut_str: &str) -> bool {
    let state = KEYBOARD_STATE.lock().unwrap();

    let parts: Vec<&str> = shortcut_str.split('+').collect();

    let mut required_ctrl = false;
    let mut required_alt = false;
    let mut required_shift = false;
    let mut required_meta = false;
    let mut main_key = "";
    
    for part in &parts {
        match part.trim() {
            "Ctrl" | "Control" => required_ctrl = true,
            "Alt" => required_alt = true,
            "Shift" => required_shift = true,
            "Win" | "Super" | "Meta" | "Cmd" | "Command" => required_meta = true,
            key_str => main_key = key_str,
        }
    }

    if state.ctrl != required_ctrl || 
       state.alt != required_alt || 
       state.shift != required_shift || 
       state.meta != required_meta {
        return false;
    }

    match_key(key, main_key)
}

// 将 rdev::Key 与字符串键名匹配
fn match_key(key: Key, key_str: &str) -> bool {
    match key_str {
        // 方向键
        "ArrowUp" | "Up" => matches!(key, Key::UpArrow),
        "ArrowDown" | "Down" => matches!(key, Key::DownArrow),
        "ArrowLeft" | "Left" => matches!(key, Key::LeftArrow),
        "ArrowRight" | "Right" => matches!(key, Key::RightArrow),
        
        // 特殊键
        "Enter" | "Return" => matches!(key, Key::Return),
        "Escape" | "Esc" => matches!(key, Key::Escape),
        "Tab" => matches!(key, Key::Tab),
        "Space" => matches!(key, Key::Space),
        "Backspace" => matches!(key, Key::Backspace),
        "Delete" => matches!(key, Key::Delete),
        
        // 字母键
        "A" => matches!(key, Key::KeyA),
        "B" => matches!(key, Key::KeyB),
        "C" => matches!(key, Key::KeyC),
        "D" => matches!(key, Key::KeyD),
        "E" => matches!(key, Key::KeyE),
        "F" => matches!(key, Key::KeyF),
        "G" => matches!(key, Key::KeyG),
        "H" => matches!(key, Key::KeyH),
        "I" => matches!(key, Key::KeyI),
        "J" => matches!(key, Key::KeyJ),
        "K" => matches!(key, Key::KeyK),
        "L" => matches!(key, Key::KeyL),
        "M" => matches!(key, Key::KeyM),
        "N" => matches!(key, Key::KeyN),
        "O" => matches!(key, Key::KeyO),
        "P" => matches!(key, Key::KeyP),
        "Q" => matches!(key, Key::KeyQ),
        "R" => matches!(key, Key::KeyR),
        "S" => matches!(key, Key::KeyS),
        "T" => matches!(key, Key::KeyT),
        "U" => matches!(key, Key::KeyU),
        "V" => matches!(key, Key::KeyV),
        "W" => matches!(key, Key::KeyW),
        "X" => matches!(key, Key::KeyX),
        "Y" => matches!(key, Key::KeyY),
        "Z" => matches!(key, Key::KeyZ),
        
        // 数字键
        "0" => matches!(key, Key::Num0),
        "1" => matches!(key, Key::Num1),
        "2" => matches!(key, Key::Num2),
        "3" => matches!(key, Key::Num3),
        "4" => matches!(key, Key::Num4),
        "5" => matches!(key, Key::Num5),
        "6" => matches!(key, Key::Num6),
        "7" => matches!(key, Key::Num7),
        "8" => matches!(key, Key::Num8),
        "9" => matches!(key, Key::Num9),
        
        // 功能键
        "F1" => matches!(key, Key::F1),
        "F2" => matches!(key, Key::F2),
        "F3" => matches!(key, Key::F3),
        "F4" => matches!(key, Key::F4),
        "F5" => matches!(key, Key::F5),
        "F6" => matches!(key, Key::F6),
        "F7" => matches!(key, Key::F7),
        "F8" => matches!(key, Key::F8),
        "F9" => matches!(key, Key::F9),
        "F10" => matches!(key, Key::F10),
        "F11" => matches!(key, Key::F11),
        "F12" => matches!(key, Key::F12),
        
        _ => false,
    }
}

// 发送导航动作事件
fn emit_navigation_action(action: &str) {
    if let Some(window) = MAIN_WINDOW_HANDLE.get() {
        let _ = window.emit(
            "navigation-action",
            serde_json::json!({
                "action": action
            }),
        );
    }
}

// 处理数字快捷键
fn handle_number_shortcut(key: Key) {
    use crate::global_state::*;

    if !NUMBER_SHORTCUTS_ENABLED.load(Ordering::SeqCst) {
        return;
    }

    let state = KEYBOARD_STATE.lock().unwrap();
    let modifier = get_number_shortcuts_modifier();

    let modifier_matches = match modifier.as_str() {
        "Ctrl" => state.ctrl && !state.shift && !state.alt && !state.meta,
        "Alt" => !state.ctrl && !state.shift && state.alt && !state.meta,
        "Shift" => !state.ctrl && state.shift && !state.alt && !state.meta,
        "Ctrl+Shift" => state.ctrl && state.shift && !state.alt && !state.meta,
        "Ctrl+Alt" => state.ctrl && !state.shift && state.alt && !state.meta,
        "Alt+Shift" => !state.ctrl && state.shift && state.alt && !state.meta,
        _ => state.ctrl && !state.shift && !state.alt && !state.meta,
    };

    if !modifier_matches {
        return;
    }

    let index = match key {
        Key::Num1 => Some(0),
        Key::Num2 => Some(1),
        Key::Num3 => Some(2),
        Key::Num4 => Some(3),
        Key::Num5 => Some(4),
        Key::Num6 => Some(5),
        Key::Num7 => Some(6),
        Key::Num8 => Some(7),
        Key::Num9 => Some(8),
        _ => None,
    };

    if let Some(index) = index {
        handle_number_paste(index);
    }
}

// 处理数字快捷键粘贴
fn handle_number_paste(index: usize) {
    if let Some(window) = MAIN_WINDOW_HANDLE.get() {
        let window_clone = window.clone();
        std::thread::spawn(move || {
            use crate::window_management::set_last_focus_hwnd;
            
            #[cfg(windows)]
            {
                use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
                let hwnd = unsafe { GetForegroundWindow() };
                set_last_focus_hwnd(hwnd.0);
            }

            let clipboard_id = match crate::database::get_clipboard_history(None) {
                Ok(items) => {
                    if index < items.len() {
                        Some(items[index].id)
                    } else {
                        None
                    }
                }
                Err(_) => None,
            };
            
            if let Some(id) = clipboard_id {
                tauri::async_runtime::spawn(async move {
                    let params = crate::services::paste_service::PasteContentParams {
                        clipboard_id: Some(id),
                        quick_text_id: None,
                    };
                    let _ = crate::commands::paste_content(params, window_clone).await;
                });
            }
        });
    }
}

// 处理Ctrl+V粘贴音效
fn handle_paste_sound(key: Key) {
    if let Key::KeyV = key {
        let state = KEYBOARD_STATE.lock().unwrap();
        if state.ctrl && !state.shift && !state.alt && !state.meta {
            std::thread::spawn(|| {
                crate::sound_manager::play_paste_sound();
            });
        }
    }
}

// 处理AI翻译取消快捷键
fn handle_translation_cancel(key: Key) {
    #[cfg(windows)]
    {
        use crate::global_state::*;

        if !AI_TRANSLATION_CANCEL_ENABLED.load(Ordering::SeqCst) {
            return;
        }

        if let Key::Escape = key {
            let state = KEYBOARD_STATE.lock().unwrap();
            if state.ctrl && state.shift && !state.alt && !state.meta {
                if let Some(window) = MAIN_WINDOW_HANDLE.get() {
                    let window_clone = window.clone();
                    std::thread::spawn(move || {
                        let _ = tauri::async_runtime::block_on(async {
                            let _ = crate::commands::cancel_translation();
                            let _ = window_clone.emit("ai-translation-cancelled", ());
                        });
                    });
                }
            }
        }
    }
}

// 处理鼠标按钮按下
fn handle_mouse_button_press(button: rdev::Button) {
    let settings = crate::settings::get_global_settings();
    
    // 中键始终检查
    if button == rdev::Button::Middle {
        if settings.app_filter_enabled {
            #[cfg(windows)]
            if !crate::app_filter::is_current_app_allowed() {
                return;
            }
        }
        handle_middle_button_press();
        return;
    }
    
    // 其他按钮需要启用鼠标监听
    if !MOUSE_MONITORING_ENABLED.load(Ordering::Relaxed) {
        return;
    }

    if settings.app_filter_enabled {
        #[cfg(windows)]
        if !crate::app_filter::is_current_app_allowed() {
            return;
        }
    }

    match button {
        rdev::Button::Left | rdev::Button::Right => handle_click_outside(),
        _ => {}
    }
}

// 处理鼠标按钮释放
fn handle_mouse_button_release(_button: rdev::Button) {
    // 可以在这里处理按钮释放事件
}

// 处理鼠标移动
fn handle_mouse_move(_x: f64, _y: f64) {
    // 可以在这里处理鼠标移动事件
}

// 处理鼠标滚轮
fn handle_mouse_wheel(_delta_x: i64, delta_y: i64) {
    // 检查是否在预览窗口上滚动
    if crate::preview_window::is_preview_window_visible() {
        let direction = if delta_y > 0 { "up" } else { "down" };
        let _ = crate::preview_window::handle_preview_scroll(direction);
    }
}

// 处理鼠标中键点击
fn handle_middle_button_press() {
    let settings = crate::settings::get_global_settings();
    if !settings.mouse_middle_button_enabled {
        return;
    }

    let state = KEYBOARD_STATE.lock().unwrap();
    
    let modifier_matches = match settings.mouse_middle_button_modifier.as_str() {
        "None" => !state.ctrl && !state.alt && !state.shift && !state.meta,
        "Ctrl" => state.ctrl && !state.alt && !state.shift && !state.meta,
        "Alt" => !state.ctrl && state.alt && !state.shift && !state.meta,
        "Shift" => !state.ctrl && !state.alt && state.shift && !state.meta,
        "Ctrl+Shift" => state.ctrl && !state.alt && state.shift && !state.meta,
        "Ctrl+Alt" => state.ctrl && state.alt && !state.shift && !state.meta,
        "Alt+Shift" => !state.ctrl && state.alt && state.shift && !state.meta,
        _ => !state.ctrl && !state.alt && !state.shift && !state.meta,
    };

    if modifier_matches {
        if let Some(window) = MAIN_WINDOW_HANDLE.get() {
            let window_clone = window.clone();
            std::thread::spawn(move || {
                crate::window_management::show_webview_window(window_clone);
            });
        }
    }
}

// 处理点击外部关闭窗口
fn handle_click_outside() {
    let is_window_pinned = crate::state_manager::is_window_pinned();
    
    if is_window_pinned {
        return;
    }

    #[cfg(windows)]
    {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        
        if let Some(window) = MAIN_WINDOW_HANDLE.get() {
            unsafe {
                let mut cursor_pos = POINT::default();
                if GetCursorPos(&mut cursor_pos).is_ok() {
                    if is_click_outside_window(window, cursor_pos) {
                        let window_clone = window.clone();
                        std::thread::spawn(move || {
                            crate::window_management::hide_webview_window(window_clone);
                        });
                    }
                }
            }
        }
    }
}

// 检查点击是否在窗口区域外
#[cfg(windows)]
fn is_click_outside_window(window: &WebviewWindow, click_point: windows::Win32::Foundation::POINT) -> bool {
    use windows::Win32::Foundation::RECT;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    if let Ok(hwnd) = window.hwnd() {
        let mut window_rect = RECT::default();
        unsafe {
            if GetWindowRect(
                windows::Win32::Foundation::HWND(hwnd.0 as isize),
                &mut window_rect,
            )
            .is_ok()
            {
                return click_point.x < window_rect.left
                    || click_point.x > window_rect.right
                    || click_point.y < window_rect.top
                    || click_point.y > window_rect.bottom;
            }
        }
    }
    true
}

