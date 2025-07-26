use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{Emitter, Manager, WebviewWindow};

#[cfg(windows)]
pub static ALT_HELD: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
pub static WINDOW_PINNED_STATE: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
pub static MAIN_WINDOW_HANDLE: OnceCell<WebviewWindow> = OnceCell::new();

#[cfg(windows)]
pub static CTRL_HELD: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
pub static SHIFT_HELD: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
pub static WIN_HELD: AtomicBool = AtomicBool::new(false);

// 数字快捷键开关
#[cfg(windows)]
pub static NUMBER_SHORTCUTS_ENABLED: AtomicBool = AtomicBool::new(true);

// AI翻译相关快捷键状态
#[cfg(windows)]
pub static AI_TRANSLATION_CANCEL_ENABLED: AtomicBool = AtomicBool::new(false);

// 预览窗口快捷键状态
#[cfg(windows)]
pub static PREVIEW_SHORTCUT_HELD: AtomicBool = AtomicBool::new(false);

// 预览窗口快捷键配置
#[cfg(windows)]
#[derive(Debug, Clone)]
pub struct PreviewShortcut {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key_code: u32,
}

#[cfg(windows)]
impl Default for PreviewShortcut {
    fn default() -> Self {
        Self {
            ctrl: true,
            shift: false,
            alt: false,
            key_code: 0xC0, // ` 键的虚拟键码
        }
    }
}

#[cfg(windows)]
static PREVIEW_SHORTCUT_CONFIG: std::sync::Mutex<PreviewShortcut> =
    std::sync::Mutex::new(PreviewShortcut {
        ctrl: true,
        shift: false,
        alt: false,
        key_code: 0xC0, // ` 键的虚拟键码
    });

// 解析预览窗口快捷键字符串
#[cfg(windows)]
pub fn parse_preview_shortcut(shortcut: &str) -> PreviewShortcut {
    let mut config = PreviewShortcut::default();

    if shortcut.is_empty() {
        return config;
    }

    let parts: Vec<&str> = shortcut.split('+').collect();

    for part in &parts[..parts.len().saturating_sub(1)] {
        match part.trim() {
            "Ctrl" => config.ctrl = true,
            "Shift" => config.shift = true,
            "Alt" => config.alt = true,
            _ => {}
        }
    }

    // 获取最后一个部分作为键
    if let Some(key) = parts.last() {
        config.key_code = match key.trim() {
            "`" => 0xC0,
            "F" => 0x46,
            "A" => 0x41,
            "B" => 0x42,
            "C" => 0x43,
            "D" => 0x44,
            "E" => 0x45,
            "G" => 0x47,
            "H" => 0x48,
            "I" => 0x49,
            "J" => 0x4A,
            "K" => 0x4B,
            "L" => 0x4C,
            "M" => 0x4D,
            "N" => 0x4E,
            "O" => 0x4F,
            "P" => 0x50,
            "Q" => 0x51,
            "R" => 0x52,
            "S" => 0x53,
            "T" => 0x54,
            "U" => 0x55,
            "V" => 0x56,
            "W" => 0x57,
            "X" => 0x58,
            "Y" => 0x59,
            "Z" => 0x5A,
            _ => 0xC0, // 默认为 ` 键
        };
    }

    config
}

// 更新预览窗口快捷键配置
#[cfg(windows)]
pub fn update_preview_shortcut(shortcut: &str) {
    let config = parse_preview_shortcut(shortcut);
    println!(
        "更新预览窗口快捷键: {} -> Ctrl:{} Shift:{} Alt:{} Key:0x{:X}",
        shortcut, config.ctrl, config.shift, config.alt, config.key_code
    );

    if let Ok(mut global_config) = PREVIEW_SHORTCUT_CONFIG.lock() {
        *global_config = config;
    }
}

// 重置预览窗口状态
#[cfg(windows)]
pub fn reset_preview_state() {
    PREVIEW_SHORTCUT_HELD.store(false, Ordering::SeqCst);
    println!("预览窗口状态已重置");
}

// 粘贴操作防抖 - 防止快速连续粘贴导致崩溃
#[cfg(windows)]
static LAST_PASTE_TIME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
#[cfg(windows)]
const PASTE_DEBOUNCE_MS: u64 = 50; // 50毫秒防抖间隔

// 鼠标监听相关的全局状态
#[cfg(windows)]
pub static MOUSE_MONITORING_ENABLED: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
static MOUSE_HOOK_HANDLE: Mutex<Option<windows::Win32::UI::WindowsAndMessaging::HHOOK>> =
    Mutex::new(None);

// 快捷键解析结构
#[cfg(windows)]
#[derive(Debug, Clone)]
struct ParsedShortcut {
    ctrl: bool,
    shift: bool,
    alt: bool,
    win: bool,
    key_code: u32,
}

// 解析快捷键字符串
#[cfg(windows)]
fn parse_shortcut(shortcut: &str) -> Option<ParsedShortcut> {
    if shortcut.is_empty() {
        return None;
    }

    let parts: Vec<&str> = shortcut.split('+').collect();
    if parts.len() < 2 {
        return None;
    }

    let mut ctrl = false;
    let mut shift = false;
    let mut alt = false;
    let mut win = false;
    let mut key_code = 0;

    for part in parts {
        match part.trim() {
            "Ctrl" => ctrl = true,
            "Shift" => shift = true,
            "Alt" => alt = true,
            "Win" => win = true,
            key => {
                // 转换按键名称到虚拟键码
                key_code = match key.to_uppercase().as_str() {
                    // 字母键
                    "A" => 0x41,
                    "B" => 0x42,
                    "C" => 0x43,
                    "D" => 0x44,
                    "E" => 0x45,
                    "F" => 0x46,
                    "G" => 0x47,
                    "H" => 0x48,
                    "I" => 0x49,
                    "J" => 0x4A,
                    "K" => 0x4B,
                    "L" => 0x4C,
                    "M" => 0x4D,
                    "N" => 0x4E,
                    "O" => 0x4F,
                    "P" => 0x50,
                    "Q" => 0x51,
                    "R" => 0x52,
                    "S" => 0x53,
                    "T" => 0x54,
                    "U" => 0x55,
                    "V" => 0x56,
                    "W" => 0x57,
                    "X" => 0x58,
                    "Y" => 0x59,
                    "Z" => 0x5A,
                    // 数字键
                    "0" => 0x30,
                    "1" => 0x31,
                    "2" => 0x32,
                    "3" => 0x33,
                    "4" => 0x34,
                    "5" => 0x35,
                    "6" => 0x36,
                    "7" => 0x37,
                    "8" => 0x38,
                    "9" => 0x39,
                    // 功能键
                    "F1" => 0x70,
                    "F2" => 0x71,
                    "F3" => 0x72,
                    "F4" => 0x73,
                    "F5" => 0x74,
                    "F6" => 0x75,
                    "F7" => 0x76,
                    "F8" => 0x77,
                    "F9" => 0x78,
                    "F10" => 0x79,
                    "F11" => 0x7A,
                    "F12" => 0x7B,
                    // 特殊键
                    "SPACE" => 0x20,
                    "ENTER" => 0x0D,
                    "TAB" => 0x09,
                    "ESCAPE" => 0x1B,
                    "BACKSPACE" => 0x08,
                    "DELETE" => 0x2E,
                    "INSERT" => 0x2D,
                    "HOME" => 0x24,
                    "END" => 0x23,
                    "PAGEUP" => 0x21,
                    "PAGEDOWN" => 0x22,
                    // 方向键
                    "LEFT" => 0x25,
                    "UP" => 0x26,
                    "RIGHT" => 0x27,
                    "DOWN" => 0x28,
                    // 符号键
                    ";" => 0xBA,
                    "=" => 0xBB,
                    "," => 0xBC,
                    "-" => 0xBD,
                    "." => 0xBE,
                    "/" => 0xBF,
                    "`" => 0xC0,
                    "[" => 0xDB,
                    "\\" => 0xDC,
                    "]" => 0xDD,
                    "'" => 0xDE,
                    _ => return None,
                };
            }
        }
    }

    if key_code == 0 {
        return None;
    }

    Some(ParsedShortcut {
        ctrl,
        shift,
        alt,
        win,
        key_code,
    })
}

#[cfg(windows)]
pub fn windows_paste() -> bool {
    use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
        KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SendMessageW, WM_PASTE};

    unsafe {
        // 确保当前窗口有焦点
        let hwnd = GetForegroundWindow();
        if hwnd == HWND(0) {
            println!("无法获取前台窗口");
            return false;
        }

        // 优先尝试 WM_PASTE
        let res = SendMessageW(hwnd, WM_PASTE, WPARAM(0), LPARAM(0));
        if res.0 != 0 {
            return true;
        }

        // 判断 Ctrl 当前是否已按下
        let ctrl_pressed = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000u16) != 0;

        // 生成按键序列
        let mut inputs: Vec<INPUT> = Vec::new();

        if !ctrl_pressed {
            // 按下 Ctrl
            let mut ctrl_down = INPUT::default();
            ctrl_down.r#type = INPUT_KEYBOARD;
            ctrl_down.Anonymous.ki = KEYBDINPUT {
                wVk: VK_CONTROL,
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: 0,
            };
            inputs.push(ctrl_down);
        }

        // 按下 V
        let mut v_down = INPUT::default();
        v_down.r#type = INPUT_KEYBOARD;
        v_down.Anonymous.ki = KEYBDINPUT {
            wVk: VK_V,
            wScan: 0,
            dwFlags: KEYBD_EVENT_FLAGS(0),
            time: 0,
            dwExtraInfo: 0,
        };
        inputs.push(v_down);

        // 释放 V
        let mut v_up = INPUT::default();
        v_up.r#type = INPUT_KEYBOARD;
        v_up.Anonymous.ki = KEYBDINPUT {
            wVk: VK_V,
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
        inputs.push(v_up);

        if !ctrl_pressed {
            // 释放 Ctrl
            let mut ctrl_up = INPUT::default();
            ctrl_up.r#type = INPUT_KEYBOARD;
            ctrl_up.Anonymous.ki = KEYBDINPUT {
                wVk: VK_CONTROL,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            inputs.push(ctrl_up);
        }

        let result = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        result != 0
    }
}

#[cfg(windows)]
unsafe extern "system" fn keyboard_hook_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        VK_CONTROL, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU,
        VK_RSHIFT, VK_RWIN, VK_SHIFT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, HC_ACTION, KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN,
        WM_SYSKEYUP,
    };

    if code == HC_ACTION as i32 {
        let kb = &*(lparam.0 as *const KBDLLHOOKSTRUCT);
        let vk = kb.vkCode as u32;

        match wparam.0 as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                // 更新 Ctrl/Alt/Shift 状态
                if vk == VK_CONTROL.0 as u32
                    || vk == VK_LCONTROL.0 as u32
                    || vk == VK_RCONTROL.0 as u32
                {
                    CTRL_HELD.store(true, Ordering::SeqCst);
                } else if vk == VK_MENU.0 as u32
                    || vk == VK_LMENU.0 as u32
                    || vk == VK_RMENU.0 as u32
                {
                    ALT_HELD.store(true, Ordering::SeqCst);
                } else if vk == VK_SHIFT.0 as u32
                    || vk == VK_LSHIFT.0 as u32
                    || vk == VK_RSHIFT.0 as u32
                {
                    SHIFT_HELD.store(true, Ordering::SeqCst);
                } else if vk == VK_LWIN.0 as u32 || vk == VK_RWIN.0 as u32 {
                    WIN_HELD.store(true, Ordering::SeqCst);
                }

                // 检查预览窗口快捷键
                if let Ok(config) = PREVIEW_SHORTCUT_CONFIG.lock() {
                    if vk == config.key_code {
                        let ctrl_held = CTRL_HELD.load(Ordering::SeqCst);
                        let shift_held = SHIFT_HELD.load(Ordering::SeqCst);
                        let alt_held = ALT_HELD.load(Ordering::SeqCst);
                        let preview_held = PREVIEW_SHORTCUT_HELD.load(Ordering::SeqCst);

                        // println!(
                        //     "预览键按下 - Ctrl: {}, Shift: {}, Alt: {}, Preview: {}",
                        //     ctrl_held, shift_held, alt_held, preview_held
                        // );

                        // 检查修饰键是否匹配配置
                        let modifiers_match = ctrl_held == config.ctrl
                            && shift_held == config.shift
                            && alt_held == config.alt;

                        if modifiers_match {
                            // 检查预览功能是否启用
                            let settings = crate::settings::get_global_settings();
                            if !settings.preview_enabled {
                                println!("预览窗口功能已禁用，忽略快捷键");
                                return LRESULT(0); // 不拦截，让系统处理
                            }

                            if !preview_held {
                                println!("检测到预览窗口快捷键，开始显示窗口");
                                PREVIEW_SHORTCUT_HELD.store(true, Ordering::SeqCst);
                            } else {
                                // println!("预览窗口快捷键已激活，跳过");
                                return LRESULT(1);
                            }

                            // 显示预览窗口
                            if let Some(window) = MAIN_WINDOW_HANDLE.get() {
                                let app_handle = window.app_handle().clone();
                                std::thread::spawn(move || {
                                    let result = tauri::async_runtime::block_on(
                                        crate::preview_window::show_preview_window(app_handle),
                                    );
                                    if let Err(e) = result {
                                        println!("显示预览窗口失败: {}", e);
                                    }
                                });
                            }

                            return LRESULT(1); // 拦截快捷键
                        }
                    }
                }

                // Ctrl + 数字 (1-9) 连续粘贴 - 添加防抖机制和开关检查
                if CTRL_HELD.load(Ordering::SeqCst)
                    && (0x31..=0x39).contains(&vk)
                    && NUMBER_SHORTCUTS_ENABLED.load(Ordering::SeqCst)
                {
                    // 防抖检查
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    let last_paste = LAST_PASTE_TIME.load(Ordering::Relaxed);
                    if now - last_paste < PASTE_DEBOUNCE_MS {
                        // 在防抖间隔内，忽略此次粘贴
                        return LRESULT(1);
                    }

                    // 更新最后粘贴时间
                    LAST_PASTE_TIME.store(now, Ordering::Relaxed);

                    let idx = (vk - 0x31) as usize; // '1' 对应索引 0
                    if let Some(window) = MAIN_WINDOW_HANDLE.get().cloned() {
                        std::thread::spawn(move || {
                            use crate::commands::paste_history_item;
                            use crate::window_management::set_last_focus_hwnd;
                            use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

                            let hwnd = unsafe { GetForegroundWindow() };
                            set_last_focus_hwnd(hwnd.0);
                            let params = crate::commands::PasteHistoryParams {
                                index: idx,
                                one_time: Some(false),
                            };
                            let _ = paste_history_item(params, window);
                        });
                    }
                    // 吞掉数字按键，避免输入到文本框
                    return LRESULT(1);
                }

                // 显示/隐藏窗口快捷键 - 从设置中读取
                let settings = crate::settings::get_global_settings();
                if !settings.toggle_shortcut.is_empty() {
                    if let Some(parsed_shortcut) = parse_shortcut(&settings.toggle_shortcut) {
                        // 检查当前按键是否匹配设置的快捷键
                        if vk == parsed_shortcut.key_code {
                            let ctrl_held = CTRL_HELD.load(Ordering::SeqCst);
                            let alt_held = ALT_HELD.load(Ordering::SeqCst);
                            let win_held = WIN_HELD.load(Ordering::SeqCst);

                            // 检查Shift状态
                            let shift_held = unsafe {
                                use windows::Win32::UI::Input::KeyboardAndMouse::{
                                    GetAsyncKeyState, VK_LSHIFT, VK_RSHIFT, VK_SHIFT,
                                };
                                (GetAsyncKeyState(VK_SHIFT.0 as i32) & 0x8000u16 as i16) != 0
                                    || (GetAsyncKeyState(VK_LSHIFT.0 as i32) & 0x8000u16 as i16)
                                        != 0
                                    || (GetAsyncKeyState(VK_RSHIFT.0 as i32) & 0x8000u16 as i16)
                                        != 0
                            };

                            // 检查修饰键是否匹配
                            if ctrl_held == parsed_shortcut.ctrl
                                && shift_held == parsed_shortcut.shift
                                && alt_held == parsed_shortcut.alt
                                && win_held == parsed_shortcut.win
                            {
                                if let Some(window) = MAIN_WINDOW_HANDLE.get().cloned() {
                                    std::thread::spawn(move || {
                                        use crate::window_management::toggle_webview_window_visibility;
                                        toggle_webview_window_visibility(window);
                                    });
                                }
                                // 拦截快捷键
                                return LRESULT(1);
                            }
                        }
                    }
                } else {
                    // 如果没有设置快捷键，使用默认的 Win + V
                    if vk == 0x56 {
                        // V键
                        let win_held = WIN_HELD.load(Ordering::SeqCst);
                        if win_held {
                            if let Some(window) = MAIN_WINDOW_HANDLE.get().cloned() {
                                std::thread::spawn(move || {
                                    use crate::window_management::toggle_webview_window_visibility;
                                    toggle_webview_window_visibility(window);
                                });
                            }
                            // 拦截快捷键
                            return LRESULT(1);
                        }
                    }
                }

                // 截屏快捷键 - 从设置中读取
                let settings = crate::settings::get_global_settings();
                if settings.screenshot_enabled && !settings.screenshot_shortcut.is_empty() {
                    if let Some(parsed_shortcut) = parse_shortcut(&settings.screenshot_shortcut) {
                        // 检查当前按键是否匹配设置的快捷键
                        if vk == parsed_shortcut.key_code {
                            let ctrl_held = CTRL_HELD.load(Ordering::SeqCst);
                            let alt_held = ALT_HELD.load(Ordering::SeqCst);
                            let win_held = WIN_HELD.load(Ordering::SeqCst);

                            // 检查Shift状态
                            let shift_held = unsafe {
                                use windows::Win32::UI::Input::KeyboardAndMouse::{
                                    GetAsyncKeyState, VK_LSHIFT, VK_RSHIFT, VK_SHIFT,
                                };
                                (GetAsyncKeyState(VK_SHIFT.0 as i32) & 0x8000u16 as i16) != 0
                                    || (GetAsyncKeyState(VK_LSHIFT.0 as i32) & 0x8000u16 as i16)
                                        != 0
                                    || (GetAsyncKeyState(VK_RSHIFT.0 as i32) & 0x8000u16 as i16)
                                        != 0
                            };

                            // 检查修饰键是否匹配
                            if ctrl_held == parsed_shortcut.ctrl
                                && shift_held == parsed_shortcut.shift
                                && alt_held == parsed_shortcut.alt
                                && win_held == parsed_shortcut.win
                            {
                                if let Some(window) = MAIN_WINDOW_HANDLE.get().cloned() {
                                    std::thread::spawn(move || {
                                        use crate::screenshot::open_screenshot_window;
                                        let app_handle = window.app_handle().clone();
                                        let _ = tauri::async_runtime::block_on(
                                            open_screenshot_window(app_handle),
                                        );
                                    });
                                }
                                // 拦截快捷键
                                return LRESULT(1);
                            }
                        }
                    }
                }

                // AI翻译取消快捷键 (Ctrl+Shift+Esc)
                if AI_TRANSLATION_CANCEL_ENABLED.load(Ordering::SeqCst) {
                    use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;

                    if vk == VK_ESCAPE.0 as u32 {
                        let ctrl_held = CTRL_HELD.load(Ordering::SeqCst);
                        let shift_held = SHIFT_HELD.load(Ordering::SeqCst);

                        if ctrl_held && shift_held {
                            println!("检测到AI翻译取消快捷键: Ctrl+Shift+Esc");

                            // 调用取消翻译
                            if let Some(window) = MAIN_WINDOW_HANDLE.get() {
                                let window_clone = window.clone();
                                std::thread::spawn(move || {
                                    let _ = tauri::async_runtime::block_on(async {
                                        let _ = crate::commands::cancel_translation();
                                        // 发送取消事件到前端
                                        let _ = window_clone.emit("ai-translation-cancelled", ());
                                    });
                                });
                            }

                            return LRESULT(1); // 拦截快捷键
                        }
                    }
                }
            }
            WM_KEYUP | WM_SYSKEYUP => {
                // 更新 Ctrl/Alt/Shift 状态
                if vk == VK_CONTROL.0 as u32
                    || vk == VK_LCONTROL.0 as u32
                    || vk == VK_RCONTROL.0 as u32
                {
                    CTRL_HELD.store(false, Ordering::SeqCst);
                } else if vk == VK_MENU.0 as u32
                    || vk == VK_LMENU.0 as u32
                    || vk == VK_RMENU.0 as u32
                {
                    ALT_HELD.store(false, Ordering::SeqCst);
                } else if vk == VK_SHIFT.0 as u32
                    || vk == VK_LSHIFT.0 as u32
                    || vk == VK_RSHIFT.0 as u32
                {
                    SHIFT_HELD.store(false, Ordering::SeqCst);
                } else if vk == VK_LWIN.0 as u32 || vk == VK_RWIN.0 as u32 {
                    WIN_HELD.store(false, Ordering::SeqCst);
                }

                // 检查预览窗口快捷键是否应该释放
                if PREVIEW_SHORTCUT_HELD.load(Ordering::SeqCst) {
                    if let Ok(config) = PREVIEW_SHORTCUT_CONFIG.lock() {
                        let ctrl_held = CTRL_HELD.load(Ordering::SeqCst);
                        let shift_held = SHIFT_HELD.load(Ordering::SeqCst);
                        let alt_held = ALT_HELD.load(Ordering::SeqCst);

                        // 检查是否有必需的修饰键被释放
                        let should_release = (config.ctrl && !ctrl_held)
                            || (config.shift && !shift_held)
                            || (config.alt && !alt_held);

                        if should_release {
                            PREVIEW_SHORTCUT_HELD.store(false, Ordering::SeqCst);

                            // 异步隐藏预览窗口并粘贴
                            std::thread::spawn(move || {
                                let _ = tauri::async_runtime::block_on(
                                    crate::preview_window::paste_current_preview_item(),
                                );
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
    // 传递给下一个钩子/系统
    CallNextHookEx(None, code, wparam, lparam)
}

#[cfg(windows)]
pub fn start_keyboard_hook() {
    use windows::Win32::Foundation::{HINSTANCE, HWND};
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, MSG, WH_KEYBOARD_LL,
    };

    std::thread::spawn(|| unsafe {
        match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), HINSTANCE(0), 0) {
            Ok(_hook) => {
                println!("键盘钩子已注册");
            }
            Err(e) => {
                println!("注册键盘钩子失败: {:?}", e);
                return;
            }
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(0), 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}

// =================== 鼠标监听功能 ===================

#[cfg(windows)]
unsafe extern "system" fn mouse_hook_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, HC_ACTION, MSLLHOOKSTRUCT, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_MOUSEWHEEL,
        WM_RBUTTONDOWN,
    };

    if code == HC_ACTION as i32 {
        // 快速检查：如果监听未启用或窗口已固定，直接返回
        if !MOUSE_MONITORING_ENABLED.load(Ordering::Relaxed)
            || WINDOW_PINNED_STATE.load(Ordering::Relaxed)
        {
            return CallNextHookEx(None, code, wparam, lparam);
        }

        // 处理鼠标事件
        match wparam.0 as u32 {
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => {
                // 获取鼠标点击位置
                let mouse_data = &*(lparam.0 as *const MSLLHOOKSTRUCT);
                let click_point = POINT {
                    x: mouse_data.pt.x,
                    y: mouse_data.pt.y,
                };

                // 检查点击是否在窗口区域外
                if let Some(window) = MAIN_WINDOW_HANDLE.get() {
                    if is_click_outside_window(window, click_point) {
                        // 在新线程中隐藏窗口，避免阻塞钩子
                        let window_clone = window.clone();
                        std::thread::spawn(move || {
                            let _ = window_clone.hide();
                        });
                    }
                }
            }
            WM_MOUSEWHEEL => {
                let preview_visible = crate::preview_window::is_preview_window_visible();
                // println!("滚轮事件检测 - 预览窗口可见: {}", preview_visible);

                // 只有在预览窗口显示时才处理滚轮事件
                if preview_visible {
                    let mouse_data = &*(lparam.0 as *const MSLLHOOKSTRUCT);
                    let wheel_delta = ((mouse_data.mouseData >> 16) & 0xFFFF) as i16;

                    // 根据滚轮方向发送滚动事件
                    let direction = if wheel_delta > 0 { "up" } else { "down" };

                    // println!("检测到滚轮事件: {} (delta: {})", direction, wheel_delta);

                    // 发送滚动事件到预览窗口
                    if let Err(e) = crate::preview_window::handle_preview_scroll(direction) {
                        println!("处理滚轮事件失败: {}", e);
                    }

                    // 拦截滚轮事件，防止传递给其他应用
                    return windows::Win32::Foundation::LRESULT(1);
                }
                // else {
                //     println!("预览窗口不可见，忽略滚轮事件");
                // }
            }
            _ => {} // 忽略其他鼠标事件
        }
    }

    CallNextHookEx(None, code, wparam, lparam)
}

#[cfg(windows)]
fn is_click_outside_window(
    window: &WebviewWindow,
    click_point: windows::Win32::Foundation::POINT,
) -> bool {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

    if let Ok(hwnd_raw) = window.hwnd() {
        let hwnd = HWND(hwnd_raw.0 as usize as isize);
        let mut rect = RECT::default();

        unsafe {
            if GetWindowRect(hwnd, &mut rect).is_ok() {
                // 检查点击位置是否在窗口矩形外
                return click_point.x < rect.left
                    || click_point.x > rect.right
                    || click_point.y < rect.top
                    || click_point.y > rect.bottom;
            }
        }
    }

    // 如果无法获取窗口矩形，默认认为在外部
    true
}

// 启用鼠标监听
#[cfg(windows)]
pub fn enable_mouse_monitoring() {
    use windows::Win32::Foundation::HINSTANCE;
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowsHookExW, WH_MOUSE_LL};

    MOUSE_MONITORING_ENABLED.store(true, Ordering::Relaxed);

    // 如果钩子还没有安装，则安装它
    let mut hook_handle = MOUSE_HOOK_HANDLE.lock().unwrap();
    if hook_handle.is_none() {
        unsafe {
            match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), HINSTANCE(0), 0) {
                Ok(hook) => {
                    *hook_handle = Some(hook);
                }
                Err(_) => {
                    // 静默处理错误
                }
            }
        }
    }
}

// 禁用鼠标监听
#[cfg(windows)]
pub fn disable_mouse_monitoring() {
    MOUSE_MONITORING_ENABLED.store(false, Ordering::Relaxed);
}

// 启用AI翻译取消快捷键监听
#[cfg(windows)]
pub fn enable_ai_translation_cancel() {
    AI_TRANSLATION_CANCEL_ENABLED.store(true, Ordering::SeqCst);
    println!("AI翻译取消快捷键已启用");
}

// 禁用AI翻译取消快捷键监听
#[cfg(windows)]
pub fn disable_ai_translation_cancel() {
    AI_TRANSLATION_CANCEL_ENABLED.store(false, Ordering::SeqCst);
    println!("AI翻译取消快捷键已禁用");
}

// 设置数字快捷键开关
#[cfg(windows)]
pub fn set_number_shortcuts_enabled(enabled: bool) {
    NUMBER_SHORTCUTS_ENABLED.store(enabled, Ordering::SeqCst);
    println!("数字快捷键设置: {}", enabled);
}

// 检查数字快捷键是否启用
#[cfg(windows)]
pub fn is_number_shortcuts_enabled() -> bool {
    NUMBER_SHORTCUTS_ENABLED.load(Ordering::SeqCst)
}

// 完全卸载鼠标钩子（用于应用退出时）
#[cfg(windows)]
pub fn uninstall_mouse_hook() {
    use windows::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx;

    MOUSE_MONITORING_ENABLED.store(false, Ordering::SeqCst);

    let mut hook_handle = MOUSE_HOOK_HANDLE.lock().unwrap();
    if let Some(hook) = hook_handle.take() {
        unsafe {
            let _ = UnhookWindowsHookEx(hook);
            println!("鼠标钩子已卸载");
        }
    }
}
