use once_cell::sync::OnceCell;
use rdev::{grab, listen, Event, EventType, Key};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{Emitter, WebviewWindow, AppHandle};

// 全局状态
pub static MAIN_WINDOW_HANDLE: OnceCell<WebviewWindow> = OnceCell::new();
static MONITORING_ACTIVE: AtomicBool = AtomicBool::new(false);
static MONITORING_THREAD_HANDLE: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);
static MOUSE_LISTENER_ACTIVE: AtomicBool = AtomicBool::new(false);
static MOUSE_LISTENER_THREAD_HANDLE: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);

// 导航键启用状态
static NAVIGATION_KEYS_ENABLED: AtomicBool = AtomicBool::new(false);

// 鼠标监听相关的全局状态
pub static MOUSE_MONITORING_ENABLED: AtomicBool = AtomicBool::new(false);

// 按键状态跟踪
static CTRL_PRESSED: AtomicBool = AtomicBool::new(false);
static ALT_PRESSED: AtomicBool = AtomicBool::new(false);
static SHIFT_PRESSED: AtomicBool = AtomicBool::new(false);
static META_PRESSED: AtomicBool = AtomicBool::new(false);

// 鼠标位置缓存
static MOUSE_POSITION: Mutex<(f64, f64)> = Mutex::new((0.0, 0.0));

// 用于发送导航动作的结构
#[derive(Clone, serde::Serialize)]
struct NavigationAction {
    action: &'static str,
}

// 启动输入监控系统
pub fn start_input_monitoring(app_handle: AppHandle, main_window: WebviewWindow) {
    if MONITORING_ACTIVE.load(Ordering::SeqCst) {
        return;
    }

    MONITORING_ACTIVE.store(true, Ordering::SeqCst);

    // 启动grab线程（用于按键和需要拦截的事件）
    let monitoring_handle = std::thread::spawn(move || {
        let callback = move |event: Event| -> Option<Event> {
            if !MONITORING_ACTIVE.load(Ordering::SeqCst) {
                return Some(event);
            }
            
            handle_input_event_with_grab(event, &app_handle, &main_window)
        };

        if let Err(error) = grab(callback) {
            eprintln!("输入监控错误: {:?}", error);
        }
    });

    if let Ok(mut handle) = MONITORING_THREAD_HANDLE.lock() {
        *handle = Some(monitoring_handle);
    }
    start_mouse_position_listener();
}

// 启动鼠标事件监听
fn start_mouse_position_listener() {
    if MOUSE_LISTENER_ACTIVE.load(Ordering::SeqCst) {
        return;
    }

    MOUSE_LISTENER_ACTIVE.store(true, Ordering::SeqCst);

    let listener_handle = std::thread::spawn(|| {
        if let Err(error) = listen(move |event| {
            if !MOUSE_LISTENER_ACTIVE.load(Ordering::SeqCst) {
                return;
            }

            match event.event_type {
                // 鼠标移动 - 更新位置缓存
                EventType::MouseMove { x, y } => {
                    if let Ok(mut pos) = MOUSE_POSITION.lock() {
                        *pos = (x, y);
                    }
                }
                // 鼠标按钮按下
                EventType::ButtonPress(button) => {
                    handle_mouse_button_press(button);
                }
                // 鼠标按钮释放
                EventType::ButtonRelease(_button) => {}
                _ => {}
            }
        }) {
            eprintln!("鼠标事件监听错误: {:?}", error);
        }
    });

    if let Ok(mut handle) = MOUSE_LISTENER_THREAD_HANDLE.lock() {
        *handle = Some(listener_handle);
    }
}

// 禁用输入事件处理
pub fn stop_input_monitoring() {
    MONITORING_ACTIVE.store(false, Ordering::SeqCst);
    MOUSE_LISTENER_ACTIVE.store(false, Ordering::SeqCst);
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

// 启用鼠标点击外部隐藏功能
pub fn enable_mouse_monitoring() {
    MOUSE_MONITORING_ENABLED.store(true, Ordering::Relaxed);
}

// 禁用鼠标点击外部隐藏功能
pub fn disable_mouse_monitoring() {
    MOUSE_MONITORING_ENABLED.store(false, Ordering::Relaxed);
}

// 检查鼠标点击外部隐藏是否启用
pub fn is_mouse_monitoring_enabled() -> bool {
    MOUSE_MONITORING_ENABLED.load(Ordering::Relaxed)
}

// 获取当前键盘修饰键状态（供其他模块使用）
pub fn get_modifier_keys_state() -> (bool, bool, bool, bool) {
    (
        CTRL_PRESSED.load(Ordering::Relaxed),
        ALT_PRESSED.load(Ordering::Relaxed),
        SHIFT_PRESSED.load(Ordering::Relaxed),
        META_PRESSED.load(Ordering::Relaxed),
    )
}

// 获取当前鼠标位置
pub fn get_mouse_position() -> Result<(i32, i32), String> {
    if let Ok(pos) = MOUSE_POSITION.lock() {
        Ok((pos.0 as i32, pos.1 as i32))
    } else {
        Err("获取鼠标位置失败".to_string())
    }
}

// 处理输入事件
fn handle_input_event_with_grab(
    event: Event,
    app_handle: &AppHandle,
    main_window: &WebviewWindow,
) -> Option<Event> {
    match event.event_type {
        // 键盘事件
        EventType::KeyPress(key) => {
            handle_key_press_with_grab(key, event, app_handle, main_window)
        }
        EventType::KeyRelease(key) => handle_key_release_with_grab(key, event),
        
        // 鼠标滚轮 
        EventType::Wheel { delta_x, delta_y } => {
            if handle_mouse_wheel(delta_x, delta_y) {
                None
            } else {
                Some(event)
            }
        }
        _ => Some(event),
    }
}

// 处理按键按下
fn handle_key_press_with_grab(
    key: Key,
    event: Event,
    app_handle: &AppHandle,
    main_window: &WebviewWindow,
) -> Option<Event> {
    // 更新修饰键状态
    match key {
        Key::ControlLeft | Key::ControlRight => CTRL_PRESSED.store(true, Ordering::Relaxed),
        Key::Alt | Key::AltGr => ALT_PRESSED.store(true, Ordering::Relaxed),
        Key::ShiftLeft | Key::ShiftRight => SHIFT_PRESSED.store(true, Ordering::Relaxed),
        Key::MetaLeft | Key::MetaRight => META_PRESSED.store(true, Ordering::Relaxed),
        _ => {}
    }

    // 检查应用过滤
    let settings = crate::settings::get_global_settings();
    if settings.app_filter_enabled {
        #[cfg(windows)]
        if !crate::app_filter::is_current_app_allowed() {
            return Some(event);
        }
    }

    // 检查主窗口是否可见
    let is_main_window_visible = main_window.is_visible().unwrap_or(false);

    // 如果主窗口可见且导航键启用，处理导航快捷键
    if is_main_window_visible && NAVIGATION_KEYS_ENABLED.load(Ordering::SeqCst) {
        if handle_navigation_hotkey(app_handle, key) {
            return None; 
        }
    }

    // 处理粘贴音效
    handle_paste_sound(key);

    // 处理翻译取消
    handle_translation_cancel(key);

    Some(event)
}

// 处理按键释放
fn handle_key_release_with_grab(key: Key, event: Event) -> Option<Event> {
    match key {
        Key::ControlLeft | Key::ControlRight => CTRL_PRESSED.store(false, Ordering::Relaxed),
        Key::Alt | Key::AltGr => ALT_PRESSED.store(false, Ordering::Relaxed),
        Key::ShiftLeft | Key::ShiftRight => SHIFT_PRESSED.store(false, Ordering::Relaxed),
        Key::MetaLeft | Key::MetaRight => META_PRESSED.store(false, Ordering::Relaxed),
        _ => {}
    }
    
    Some(event)
}

// 处理导航热键
fn handle_navigation_hotkey(app_handle: &AppHandle, key: Key) -> bool {
    // 检查翻译进行状态
    #[cfg(windows)]
    if crate::global_state::TRANSLATION_IN_PROGRESS.load(Ordering::SeqCst) {
        return false;
    }

    // 从配置读取快捷键设置
    let settings = crate::settings::get_global_settings();
    
    let shortcuts = [
        (&settings.navigate_up_shortcut, "navigate-up"),
        (&settings.navigate_down_shortcut, "navigate-down"),
        (&settings.tab_left_shortcut, "tab-left"),
        (&settings.tab_right_shortcut, "tab-right"),
        (&settings.focus_search_shortcut, "focus-search"),
        (&settings.hide_window_shortcut, "hide-window"),
        (&settings.execute_item_shortcut, "execute-item"),
        (&settings.previous_group_shortcut, "previous-group"),
        (&settings.next_group_shortcut, "next-group"),
        (&settings.toggle_pin_shortcut, "toggle-pin"),
    ];

    for (shortcut_str, action) in shortcuts {
        if check_shortcut_match(key, shortcut_str) {
            let _ = app_handle.emit("navigation-action", NavigationAction { action });
            return true;
        }
    }
    
    false
}

// 检查当前按键是否匹配快捷键字符串
fn check_shortcut_match(key: Key, shortcut_str: &str) -> bool {
    let ctrl = CTRL_PRESSED.load(Ordering::Relaxed);
    let alt = ALT_PRESSED.load(Ordering::Relaxed);
    let shift = SHIFT_PRESSED.load(Ordering::Relaxed);
    let meta = META_PRESSED.load(Ordering::Relaxed);

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

    if ctrl != required_ctrl || 
       alt != required_alt || 
       shift != required_shift || 
       meta != required_meta {
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
        "Home" => matches!(key, Key::Home),
        "End" => matches!(key, Key::End),
        "PageUp" => matches!(key, Key::PageUp),
        "PageDown" => matches!(key, Key::PageDown),
        
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


// 处理Ctrl+V粘贴音效
fn handle_paste_sound(key: Key) {
    if let Key::KeyV = key {
        let ctrl = CTRL_PRESSED.load(Ordering::Relaxed);
        let shift = SHIFT_PRESSED.load(Ordering::Relaxed);
        let alt = ALT_PRESSED.load(Ordering::Relaxed);
        let meta = META_PRESSED.load(Ordering::Relaxed);
        
        if ctrl && !shift && !alt && !meta {
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
            let ctrl = CTRL_PRESSED.load(Ordering::Relaxed);
            let shift = SHIFT_PRESSED.load(Ordering::Relaxed);
            let alt = ALT_PRESSED.load(Ordering::Relaxed);
            let meta = META_PRESSED.load(Ordering::Relaxed);
            
            if ctrl && shift && !alt && !meta {
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

// 处理鼠标滚轮
fn handle_mouse_wheel(_delta_x: i64, delta_y: i64) -> bool {
    if crate::preview_window::is_preview_window_visible() {
        let direction = if delta_y > 0 { "up" } else { "down" };
        let _ = crate::preview_window::handle_preview_scroll(direction);
        return true;
    }
    false
}

// 处理鼠标中键点击
fn handle_middle_button_press() {
    let settings = crate::settings::get_global_settings();
    if !settings.mouse_middle_button_enabled {
        return;
    }

    let ctrl = CTRL_PRESSED.load(Ordering::Relaxed);
    let shift = SHIFT_PRESSED.load(Ordering::Relaxed);
    let alt = ALT_PRESSED.load(Ordering::Relaxed);
    let meta = META_PRESSED.load(Ordering::Relaxed);
    
    let modifier_matches = match settings.mouse_middle_button_modifier.as_str() {
        "None" => !ctrl && !alt && !shift && !meta,
        "Ctrl" => ctrl && !alt && !shift && !meta,
        "Alt" => !ctrl && alt && !shift && !meta,
        "Shift" => !ctrl && !alt && shift && !meta,
        "Ctrl+Shift" => ctrl && !alt && shift && !meta,
        "Ctrl+Alt" => ctrl && alt && !shift && !meta,
        "Alt+Shift" => !ctrl && alt && shift && !meta,
        _ => !ctrl && !alt && !shift && !meta,
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

    // 从 rdev 缓存获取鼠标位置
    if let Ok((cursor_x, cursor_y)) = get_mouse_position() {
        if let Some(window) = MAIN_WINDOW_HANDLE.get() {
            if is_click_outside_window(window, cursor_x, cursor_y) {
                let window_clone = window.clone();
                std::thread::spawn(move || {
                    crate::window_management::hide_webview_window(window_clone);
                });
            }
        }
    }
}

// 检查点击是否在窗口区域外
fn is_click_outside_window(window: &WebviewWindow, click_x: i32, click_y: i32) -> bool {
    if let (Ok(position), Ok(size)) = (window.outer_position(), window.outer_size()) {
        let window_x = position.x;
        let window_y = position.y;
        let window_width = size.width as i32;
        let window_height = size.height as i32;
        
        // 检查点击是否在窗口外
        return click_x < window_x
            || click_x > window_x + window_width
            || click_y < window_y
            || click_y > window_y + window_height;
    }
    true
}

