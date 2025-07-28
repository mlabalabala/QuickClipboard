use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_ESCAPE, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU,
    VK_RCONTROL, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
};

// 全局状态
static POLLING_ACTIVE: AtomicBool = AtomicBool::new(false);
static POLLING_THREAD_HANDLE: Mutex<Option<std::thread::JoinHandle<()>>> = Mutex::new(None);

// 按键状态结构
#[derive(Debug, Clone, Copy)]
struct KeyState {
    ctrl: bool,
    alt: bool,
    shift: bool,
    win: bool,
    escape: bool,
    // 数字键 1-9
    num1: bool,
    num2: bool,
    num3: bool,
    num4: bool,
    num5: bool,
    num6: bool,
    num7: bool,
    num8: bool,
    num9: bool,
    // 字母键
    a: bool,
    c: bool,
    v: bool,
    // 功能键
    f1: bool,
    f2: bool,
    f3: bool,
    f4: bool,
    f5: bool,
    f6: bool,
    f7: bool,
    f8: bool,
    f9: bool,
    f10: bool,
    f11: bool,
    f12: bool,

    // 预览快捷键（默认反引号）
    backtick: bool,
}

impl Default for KeyState {
    fn default() -> Self {
        Self {
            ctrl: false,
            alt: false,
            shift: false,
            win: false,
            escape: false,
            num1: false,
            num2: false,
            num3: false,
            num4: false,
            num5: false,
            num6: false,
            num7: false,
            num8: false,
            num9: false,
            a: false,
            c: false,
            v: false,
            f1: false,
            f2: false,
            f3: false,
            f4: false,
            f5: false,
            f6: false,
            f7: false,
            f8: false,
            f9: false,
            f10: false,
            f11: false,
            f12: false,

            backtick: false,
        }
    }
}

// 基于定时器的按键检测系统
pub fn start_keyboard_polling_system() {
    if POLLING_ACTIVE.load(Ordering::SeqCst) {
        return;
    }

    let polling_handle = std::thread::spawn(|| {
        POLLING_ACTIVE.store(true, Ordering::SeqCst);

        let mut last_state = KeyState::default();

        // 主轮询循环，每15ms检查一次按键状态
        while POLLING_ACTIVE.load(Ordering::SeqCst) {
            let current_state = get_current_key_state();

            // 处理所有按键状态变化
            handle_key_state_changes(&last_state, &current_state);

            last_state = current_state;

            // 等待15ms再进行下一次检测
            std::thread::sleep(Duration::from_millis(15));
        }
    });

    if let Ok(mut handle) = POLLING_THREAD_HANDLE.lock() {
        *handle = Some(polling_handle);
    }
}

// 获取当前按键状态
fn get_current_key_state() -> KeyState {
    unsafe {
        KeyState {
            // 修饰键检测
            ctrl: (GetAsyncKeyState(VK_CONTROL.0 as i32) & 0x8000u16 as i16) != 0
                || (GetAsyncKeyState(VK_LCONTROL.0 as i32) & 0x8000u16 as i16) != 0
                || (GetAsyncKeyState(VK_RCONTROL.0 as i32) & 0x8000u16 as i16) != 0,

            alt: (GetAsyncKeyState(VK_MENU.0 as i32) & 0x8000u16 as i16) != 0
                || (GetAsyncKeyState(VK_LMENU.0 as i32) & 0x8000u16 as i16) != 0
                || (GetAsyncKeyState(VK_RMENU.0 as i32) & 0x8000u16 as i16) != 0,

            shift: (GetAsyncKeyState(VK_SHIFT.0 as i32) & 0x8000u16 as i16) != 0
                || (GetAsyncKeyState(VK_LSHIFT.0 as i32) & 0x8000u16 as i16) != 0
                || (GetAsyncKeyState(VK_RSHIFT.0 as i32) & 0x8000u16 as i16) != 0,

            win: (GetAsyncKeyState(VK_LWIN.0 as i32) & 0x8000u16 as i16) != 0
                || (GetAsyncKeyState(VK_RWIN.0 as i32) & 0x8000u16 as i16) != 0,

            escape: (GetAsyncKeyState(VK_ESCAPE.0 as i32) & 0x8000u16 as i16) != 0,

            // 数字键检测
            num1: (GetAsyncKeyState(0x31) & 0x8000u16 as i16) != 0,
            num2: (GetAsyncKeyState(0x32) & 0x8000u16 as i16) != 0,
            num3: (GetAsyncKeyState(0x33) & 0x8000u16 as i16) != 0,
            num4: (GetAsyncKeyState(0x34) & 0x8000u16 as i16) != 0,
            num5: (GetAsyncKeyState(0x35) & 0x8000u16 as i16) != 0,
            num6: (GetAsyncKeyState(0x36) & 0x8000u16 as i16) != 0,
            num7: (GetAsyncKeyState(0x37) & 0x8000u16 as i16) != 0,
            num8: (GetAsyncKeyState(0x38) & 0x8000u16 as i16) != 0,
            num9: (GetAsyncKeyState(0x39) & 0x8000u16 as i16) != 0,

            // 字母键检测
            a: (GetAsyncKeyState(0x41) & 0x8000u16 as i16) != 0,
            c: (GetAsyncKeyState(0x43) & 0x8000u16 as i16) != 0,
            v: (GetAsyncKeyState(0x56) & 0x8000u16 as i16) != 0,

            // 功能键检测
            f1: (GetAsyncKeyState(0x70) & 0x8000u16 as i16) != 0,
            f2: (GetAsyncKeyState(0x71) & 0x8000u16 as i16) != 0,
            f3: (GetAsyncKeyState(0x72) & 0x8000u16 as i16) != 0,
            f4: (GetAsyncKeyState(0x73) & 0x8000u16 as i16) != 0,
            f5: (GetAsyncKeyState(0x74) & 0x8000u16 as i16) != 0,
            f6: (GetAsyncKeyState(0x75) & 0x8000u16 as i16) != 0,
            f7: (GetAsyncKeyState(0x76) & 0x8000u16 as i16) != 0,
            f8: (GetAsyncKeyState(0x77) & 0x8000u16 as i16) != 0,
            f9: (GetAsyncKeyState(0x78) & 0x8000u16 as i16) != 0,
            f10: (GetAsyncKeyState(0x79) & 0x8000u16 as i16) != 0,
            f11: (GetAsyncKeyState(0x7A) & 0x8000u16 as i16) != 0,
            f12: (GetAsyncKeyState(0x7B) & 0x8000u16 as i16) != 0,

            // 反引号键检测
            backtick: (GetAsyncKeyState(0xC0) & 0x8000u16 as i16) != 0,
        }
    }
}

// 处理按键状态变化
fn handle_key_state_changes(last_state: &KeyState, current_state: &KeyState) {
    use crate::global_state::*;

    // 更新全局修饰键状态
    CTRL_HELD.store(current_state.ctrl, Ordering::SeqCst);
    ALT_HELD.store(current_state.alt, Ordering::SeqCst);
    SHIFT_HELD.store(current_state.shift, Ordering::SeqCst);
    WIN_HELD.store(current_state.win, Ordering::SeqCst);

    // 处理主窗口显示快捷键 (Win+V 或其他设置的快捷键)
    handle_main_window_shortcut_change(last_state, current_state);

    // 处理预览窗口快捷键 (Ctrl+`)
    handle_preview_shortcut_change(last_state, current_state);

    // 处理数字快捷键
    handle_number_shortcuts_change(last_state, current_state);

    // 处理其他快捷键
    handle_other_shortcuts_change(last_state, current_state);

    // 处理AI翻译取消快捷键 (Ctrl+Shift+Esc)
    handle_ai_translation_cancel_change(last_state, current_state);
}

// 处理主窗口显示快捷键变化
fn handle_main_window_shortcut_change(last_state: &KeyState, current_state: &KeyState) {
    use crate::global_state::*;

    // 获取当前设置的主窗口快捷键
    let settings = crate::settings::get_global_settings();
    let toggle_shortcut = if settings.toggle_shortcut.is_empty() {
        "Win+V".to_string()
    } else {
        settings.toggle_shortcut.clone()
    };

    // 解析快捷键
    if let Some(parsed_shortcut) = parse_shortcut(&toggle_shortcut) {
        let last_combo = check_main_window_shortcut_combo(last_state, &parsed_shortcut);
        let current_combo = check_main_window_shortcut_combo(current_state, &parsed_shortcut);

        // 检测快捷键按下（从未按下到按下）
        if !last_combo && current_combo {
            // 显示主窗口
            if let Some(window) = crate::mouse_hook::MAIN_WINDOW_HANDLE.get() {
                let window_clone = window.clone();
                std::thread::spawn(move || {
                    crate::window_management::toggle_webview_window_visibility(
                        window_clone.clone(),
                    );

                    // 延迟一点时间等窗口显示完成，然后模拟点击窗口
                    std::thread::sleep(std::time::Duration::from_millis(100));

                    #[cfg(windows)]
                    {
                        crate::window_management::simulate_click_on_window(&window_clone);
                    }
                });
            }
        }
    }
}

// 检查主窗口快捷键组合是否匹配
fn check_main_window_shortcut_combo(
    state: &KeyState,
    config: &crate::global_state::ParsedShortcut,
) -> bool {
    let ctrl_match = state.ctrl == config.ctrl;
    let shift_match = state.shift == config.shift;
    let alt_match = state.alt == config.alt;
    let win_match = state.win == config.win;

    // 检查主键
    let key_match = match config.key_code {
        0x56 => state.v,   // V 键
        0x41 => state.a,   // A 键
        0x43 => state.c,   // C 键
        0x70 => state.f1,  // F1 键
        0x71 => state.f2,  // F2 键
        0x72 => state.f3,  // F3 键
        0x73 => state.f4,  // F4 键
        0x74 => state.f5,  // F5 键
        0x75 => state.f6,  // F6 键
        0x76 => state.f7,  // F7 键
        0x77 => state.f8,  // F8 键
        0x78 => state.f9,  // F9 键
        0x79 => state.f10, // F10 键
        0x7A => state.f11, // F11 键
        0x7B => state.f12, // F12 键
        _ => false,        // 其他键暂时不支持
    };

    ctrl_match && shift_match && alt_match && win_match && key_match
}

// 处理预览窗口快捷键变化
fn handle_preview_shortcut_change(last_state: &KeyState, current_state: &KeyState) {
    use crate::global_state::*;

    // 检查预览快捷键配置
    if let Ok(config) = PREVIEW_SHORTCUT_CONFIG.lock() {
        let last_combo = check_shortcut_combo(last_state, &config);
        let current_combo = check_shortcut_combo(current_state, &config);

        // 检测快捷键按下
        if !last_combo && current_combo {
            let settings = crate::settings::get_global_settings();
            if !settings.preview_enabled {
                return;
            }

            PREVIEW_SHORTCUT_HELD.store(true, Ordering::SeqCst);

            // 显示预览窗口
            if let Some(window) = crate::mouse_hook::MAIN_WINDOW_HANDLE.get() {
                let app_handle = window.app_handle().clone();
                std::thread::spawn(move || {
                    let _ = tauri::async_runtime::block_on(
                        crate::preview_window::show_preview_window(app_handle),
                    );
                });
            }
        }
        // 检测快捷键释放
        else if last_combo && !current_combo {
            PREVIEW_SHORTCUT_HELD.store(false, Ordering::SeqCst);

            // 隐藏预览窗口并粘贴
            std::thread::spawn(move || {
                let _ = tauri::async_runtime::block_on(
                    crate::preview_window::paste_current_preview_item(),
                );
            });
        }
    }
}

// 检查快捷键组合是否匹配
fn check_shortcut_combo(state: &KeyState, config: &crate::global_state::PreviewShortcut) -> bool {
    let ctrl_match = state.ctrl == config.ctrl;
    let shift_match = state.shift == config.shift;
    let alt_match = state.alt == config.alt;

    // 检查主键
    let key_match = match config.key_code {
        0xC0 => state.backtick, // 反引号
        _ => false,             // 其他键暂时不支持
    };

    ctrl_match && shift_match && alt_match && key_match
}

// 处理数字快捷键变化
fn handle_number_shortcuts_change(last_state: &KeyState, current_state: &KeyState) {
    use crate::global_state::*;

    if !NUMBER_SHORTCUTS_ENABLED.load(Ordering::SeqCst) || !current_state.ctrl {
        return;
    }

    // 检查数字键按下事件
    let numbers = [
        (last_state.num1, current_state.num1, 0),
        (last_state.num2, current_state.num2, 1),
        (last_state.num3, current_state.num3, 2),
        (last_state.num4, current_state.num4, 3),
        (last_state.num5, current_state.num5, 4),
        (last_state.num6, current_state.num6, 5),
        (last_state.num7, current_state.num7, 6),
        (last_state.num8, current_state.num8, 7),
        (last_state.num9, current_state.num9, 8),
    ];

    for (last_pressed, current_pressed, index) in numbers {
        if !last_pressed && current_pressed {
            handle_number_shortcut_paste(index);
        }
    }
}

// 处理数字快捷键粘贴
fn handle_number_shortcut_paste(index: usize) {
    use crate::mouse_hook::MAIN_WINDOW_HANDLE;

    // 防抖检查
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    static LAST_PASTE_TIME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    const PASTE_DEBOUNCE_MS: u64 = 50;

    let last_paste = LAST_PASTE_TIME.load(Ordering::Relaxed);
    if now - last_paste < PASTE_DEBOUNCE_MS {
        return;
    }

    LAST_PASTE_TIME.store(now, Ordering::Relaxed);

    if let Some(window) = MAIN_WINDOW_HANDLE.get().cloned() {
        std::thread::spawn(move || {
            use crate::commands::paste_history_item;
            use crate::window_management::set_last_focus_hwnd;
            use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

            let hwnd = unsafe { GetForegroundWindow() };
            set_last_focus_hwnd(hwnd.0);
            let params = crate::commands::PasteHistoryParams {
                index,
                one_time: Some(false),
            };
            let _ = paste_history_item(params, window);
        });
    }
}

// 处理其他快捷键变化
fn handle_other_shortcuts_change(_last_state: &KeyState, _current_state: &KeyState) {
    // 这里可以添加其他快捷键的处理逻辑
    // 比如窗口切换快捷键、截屏快捷键等
}

// 处理AI翻译取消快捷键变化
fn handle_ai_translation_cancel_change(last_state: &KeyState, current_state: &KeyState) {
    use crate::global_state::*;

    if !AI_TRANSLATION_CANCEL_ENABLED.load(Ordering::SeqCst) {
        return;
    }

    let last_combo = last_state.ctrl && last_state.shift && last_state.escape;
    let current_combo = current_state.ctrl && current_state.shift && current_state.escape;

    if !last_combo && current_combo {
        if let Some(window) = crate::mouse_hook::MAIN_WINDOW_HANDLE.get() {
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

// 停止按键轮询系统
pub fn stop_keyboard_polling_system() {
    if !POLLING_ACTIVE.load(Ordering::SeqCst) {
        return;
    }

    // 设置停止标志
    POLLING_ACTIVE.store(false, Ordering::SeqCst);

    // 等待线程结束
    if let Ok(mut handle) = POLLING_THREAD_HANDLE.lock() {
        if let Some(thread_handle) = handle.take() {
            let _ = thread_handle.join();
        }
    }
}

// 检查轮询系统是否活跃
pub fn is_polling_system_active() -> bool {
    POLLING_ACTIVE.load(Ordering::SeqCst)
}
