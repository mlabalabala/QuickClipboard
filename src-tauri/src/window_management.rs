use crate::mouse_hook::{disable_mouse_monitoring, enable_mouse_monitoring, WINDOW_PINNED_STATE};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::WebviewWindow;

static MAIN_WINDOW_AUTO_SHOWN: AtomicBool = AtomicBool::new(false);

pub fn toggle_webview_window_visibility(window: tauri::WebviewWindow) {
    if window.is_visible().unwrap_or(true) {
        // 发送隐藏动画事件给前端
        {
            use tauri::Emitter;
            let _ = window.emit("window-hide-animation", ());
            println!("发送隐藏动画事件 (webview)");
        }

        // 等待动画完成后再隐藏窗口
        std::thread::sleep(std::time::Duration::from_millis(300));

        // 隐藏窗口前恢复焦点并停止鼠标监听
        let _ = restore_last_focus();
        let _ = window.hide();
        #[cfg(windows)]
        disable_mouse_monitoring();
        // 禁用导航按键监听
        #[cfg(windows)]
        crate::shortcut_interceptor::disable_navigation_keys();
    } else {
        // 智能定位窗口到光标位置
        #[cfg(windows)]
        {
            let _ = position_window_at_cursor(&window);
        }

        // 显示窗口但不抢占焦点
        let _ = window.show();

        // 发送显示动画事件给前端
        {
            use tauri::Emitter;
            let _ = window.emit("window-show-animation", ());
            println!("发送显示动画事件 (webview)");
        }

        // 确保窗口设置为工具窗口（不抢占焦点）
        #[cfg(windows)]
        {
            let _ = set_tool_window(&window);
            // 将窗口设置为前台但不抢夺焦点
            bring_window_to_front_without_focus(&window);
            // 启用导航按键监听
            crate::shortcut_interceptor::enable_navigation_keys();
        }
        #[cfg(windows)]
        enable_mouse_monitoring();
    }
}

#[cfg(windows)]
static mut LAST_FOCUS_HWND: Option<isize> = None;
#[cfg(windows)]
static LAST_FOCUS_MUTEX: Mutex<()> = Mutex::new(());

#[cfg(windows)]
pub fn set_last_focus_hwnd(hwnd_val: isize) {
    let _lock = LAST_FOCUS_MUTEX.lock().unwrap();
    unsafe {
        LAST_FOCUS_HWND = Some(hwnd_val);
    }
}

pub fn focus_clipboard_window(window: WebviewWindow) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};
        let _lock = LAST_FOCUS_MUTEX.lock().unwrap();
        unsafe {
            // 记录当前前台窗口
            let hwnd = GetForegroundWindow();
            if hwnd.0 != 0 {
                LAST_FOCUS_HWND = Some(hwnd.0);
            }
            // 让剪贴板窗口获得焦点
            if let Ok(hwnd_raw) = window.hwnd() {
                let hwnd_clip = HWND(hwnd_raw.0 as usize as isize);
                let _ = SetForegroundWindow(hwnd_clip);
            }
        }
    }
    Ok(())
}

pub fn restore_last_focus() -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
        let _lock = LAST_FOCUS_MUTEX.lock().unwrap();
        unsafe {
            if let Some(hwnd_val) = LAST_FOCUS_HWND {
                let hwnd = HWND(hwnd_val);
                let _ = SetForegroundWindow(hwnd);
                LAST_FOCUS_HWND = None;
            }
        }
    }
    Ok(())
}

// 设置窗口固定状态（控制粘贴后是否隐藏窗口）
pub fn set_window_pinned(pinned: bool) -> Result<(), String> {
    // 更新全局固定状态
    WINDOW_PINNED_STATE.store(pinned, Ordering::SeqCst);
    Ok(())
}

// 获取窗口固定状态
pub fn get_window_pinned() -> bool {
    WINDOW_PINNED_STATE.load(Ordering::SeqCst)
}

// 设置主窗口自动显示状态
pub fn set_main_window_auto_shown(auto_shown: bool) {
    MAIN_WINDOW_AUTO_SHOWN.store(auto_shown, Ordering::SeqCst);
}

// 获取主窗口自动显示状态
pub fn get_main_window_auto_shown() -> bool {
    MAIN_WINDOW_AUTO_SHOWN.load(Ordering::SeqCst)
}

// 检查当前前台窗口是否是自己的应用窗口
#[cfg(windows)]
pub fn is_current_window_own_app(window: &tauri::WebviewWindow) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    unsafe {
        let foreground_hwnd = GetForegroundWindow();
        if let Ok(app_hwnd_raw) = window.hwnd() {
            let app_hwnd = HWND(app_hwnd_raw.0 as usize as isize);
            return foreground_hwnd == app_hwnd;
        }
    }
    false
}

#[cfg(not(windows))]
pub fn is_current_window_own_app(_window: &tauri::WebviewWindow) -> bool {
    false
}

// 检查我们的窗口是否应该接收导航按键（简单检查：只要可见就接收）
#[cfg(windows)]
pub fn should_receive_navigation_keys(window: &tauri::WebviewWindow) -> bool {
    // 简单策略：只要窗口可见就接收导航按键
    // 这样可以确保用户在使用我们的窗口时能够进行键盘导航
    window.is_visible().unwrap_or(false)
}

#[cfg(not(windows))]
pub fn should_receive_navigation_keys(_window: &tauri::WebviewWindow) -> bool {
    false
}

// 激活窗口，用于隐藏系统剪贴板
#[cfg(windows)]
pub fn simulate_click_on_window(window: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{BringWindowToTop, SetForegroundWindow};

    if let Ok(hwnd_raw) = window.hwnd() {
        let hwnd = HWND(hwnd_raw.0 as usize as isize);

        unsafe {
            // 方法1：设置为前台窗口
            let _ = SetForegroundWindow(hwnd);

            // 方法2：将窗口置顶
            let _ = BringWindowToTop(hwnd);
        }
    }
}

// 将窗口设置为前台但不抢夺焦点
#[cfg(windows)]
pub fn bring_window_to_front_without_focus(window: &tauri::WebviewWindow) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, SetWindowPos, HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    };

    if let Ok(hwnd_raw) = window.hwnd() {
        let hwnd = HWND(hwnd_raw.0 as usize as isize);

        unsafe {
            // 使用 SetWindowPos 将窗口置于前台但不激活（不抢夺焦点）
            let _ = SetWindowPos(
                hwnd,
                HWND_TOP,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );

            // 备用方法：BringWindowToTop（某些情况下可能仍会抢夺焦点，但通常比 SetForegroundWindow 温和）
            let _ = BringWindowToTop(hwnd);
        }
    }
}

#[cfg(not(windows))]
pub fn simulate_click_on_window(_window: &tauri::WebviewWindow) {
    // 非Windows平台暂不实现
}

#[cfg(not(windows))]
pub fn bring_window_to_front_without_focus(_window: &tauri::WebviewWindow) {
    // 非Windows平台暂不实现
}

// 如果主窗口是自动显示的，则隐藏它
pub fn hide_main_window_if_auto_shown(window: &WebviewWindow) -> Result<(), String> {
    if MAIN_WINDOW_AUTO_SHOWN.load(Ordering::SeqCst) {
        // 重置自动显示状态
        MAIN_WINDOW_AUTO_SHOWN.store(false, Ordering::SeqCst);

        // 隐藏主窗口并停止鼠标监听
        let _ = restore_last_focus();
        window
            .hide()
            .map_err(|e| format!("隐藏主窗口失败: {}", e))?;

        #[cfg(windows)]
        disable_mouse_monitoring();

        println!("主窗口已隐藏（因设置窗口关闭）");
    }
    Ok(())
}

// 设置窗口为工具窗口，避免抢占焦点
#[cfg(windows)]
pub fn set_tool_window(window: &WebviewWindow) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST,
        SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    if let Ok(hwnd_raw) = window.hwnd() {
        let hwnd = HWND(hwnd_raw.0 as usize as isize);

        unsafe {
            // 获取当前扩展样式
            let current_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);

            // 添加工具窗口和无激活样式
            let new_style =
                current_style | (WS_EX_TOOLWINDOW.0 as isize) | (WS_EX_NOACTIVATE.0 as isize);

            // 设置新的扩展样式
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);

            // 设置窗口始终置顶且不激活
            let _ = SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_FRAMECHANGED | SWP_NOZORDER,
            );
        }

        // 禁用窗口阴影
        disable_window_shadow(window)?;

        Ok(())
    } else {
        Err("获取窗口句柄失败".to_string())
    }
}

// 设置超级置顶窗口
#[cfg(windows)]
pub fn set_super_topmost_window(window: &WebviewWindow) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST,
        SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    };

    if let Ok(hwnd_raw) = window.hwnd() {
        let hwnd = HWND(hwnd_raw.0 as usize as isize);

        unsafe {
            // 获取当前扩展样式
            let current_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);

            // 添加最高级别的置顶样式
            let new_style = current_style
                | (WS_EX_TOOLWINDOW.0 as isize)   // 工具窗口
                | (WS_EX_NOACTIVATE.0 as isize)   // 不激活
                | (WS_EX_TOPMOST.0 as isize); // 扩展置顶样式

            // 设置新的扩展样式
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);

            // 第一次设置为置顶
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
            );

            // 强制刷新窗口层级
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }

        println!("窗口已设置为超级置顶，应该能显示在开始菜单之上");

        // 禁用窗口阴影
        disable_window_shadow(window)?;

        Ok(())
    } else {
        Err("无法获取窗口句柄".to_string())
    }
}

// 禁用窗口阴影
#[cfg(windows)]
pub fn disable_window_shadow(window: &WebviewWindow) -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_NCRENDERING_POLICY, DWMWA_WINDOW_CORNER_PREFERENCE,
    };

    if let Ok(hwnd_raw) = window.hwnd() {
        let hwnd = HWND(hwnd_raw.0 as usize as isize);

        unsafe {
            // 禁用非客户区渲染（包括阴影）
            let policy: u32 = 2; // DWMNCRP_DISABLED
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_NCRENDERING_POLICY,
                &policy as *const u32 as *const std::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            );

            // 设置窗口圆角为不圆角（这也有助于移除阴影）
            let corner_preference: u32 = 1; // DWMWCP_DONOTROUND
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &corner_preference as *const u32 as *const std::ffi::c_void,
                std::mem::size_of::<u32>() as u32,
            );

            println!("窗口阴影已禁用");
        }

        Ok(())
    } else {
        Err("获取窗口句柄失败".to_string())
    }
}

#[cfg(not(windows))]
pub fn set_tool_window(_window: &WebviewWindow) -> Result<(), String> {
    // 非Windows平台暂不实现
    Ok(())
}

// 获取文本插入符位置并智能定位窗口
#[cfg(windows)]
pub fn position_window_at_cursor(window: &WebviewWindow) -> Result<(), String> {
    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::Graphics::Gdi::{
        ClientToScreen, GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetCaretPos, GetCursorPos, GetForegroundWindow, GetGUIThreadInfo, GetSystemMetrics,
        GetWindowRect, GetWindowThreadProcessId, GUITHREADINFO, GUITHREADINFO_FLAGS, SM_CXSCREEN,
        SM_CYSCREEN,
    };

    unsafe {
        // 多种方法尝试获取插入符位置
        let mut caret_pos = POINT { x: 0, y: 0 };
        let mut use_caret = false;
        let mut caret_source = "unknown";

        // 获取插入符位置的主要方法
        if !use_caret {
            let foreground_window = GetForegroundWindow();
            if foreground_window.0 != 0 {
                let thread_id = GetWindowThreadProcessId(foreground_window, None);

                // 获取窗口类名和标题，用于识别浏览器/WebView窗口
                let window_class = get_window_class_name(foreground_window);
                let window_title = get_window_title(foreground_window);

                // 检查是否为浏览器或WebView窗口
                let is_browser_or_webview =
                    is_browser_or_webview_window(&window_class, &window_title);

                if is_browser_or_webview {
                    // 对于浏览器/WebView窗口，使用鼠标位置作为基准
                    if GetCursorPos(&mut caret_pos).is_ok() {
                        use_caret = true;
                        caret_source = "browser_mouse_position";
                    }
                } else {
                    // 对于非浏览器窗口，尝试获取真实插入符位置
                    let mut gui_info = GUITHREADINFO {
                        cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
                        flags: GUITHREADINFO_FLAGS(0),
                        hwndActive: windows::Win32::Foundation::HWND(0),
                        hwndFocus: windows::Win32::Foundation::HWND(0),
                        hwndCapture: windows::Win32::Foundation::HWND(0),
                        hwndMenuOwner: windows::Win32::Foundation::HWND(0),
                        hwndMoveSize: windows::Win32::Foundation::HWND(0),
                        hwndCaret: windows::Win32::Foundation::HWND(0),
                        rcCaret: RECT {
                            left: 0,
                            top: 0,
                            right: 0,
                            bottom: 0,
                        },
                    };

                    if GetGUIThreadInfo(thread_id, &mut gui_info).is_ok() {
                        // 检查是否有有效的插入符信息
                        if gui_info.hwndCaret.0 != 0 {
                            // 使用插入符矩形的左下角作为基准点
                            caret_pos.x = gui_info.rcCaret.left;
                            caret_pos.y = gui_info.rcCaret.bottom;

                            // 将客户端坐标转换为屏幕坐标
                            if ClientToScreen(gui_info.hwndCaret, &mut caret_pos).as_bool() {
                                use_caret = true;
                                caret_source = "gui_thread_caret";
                            }
                        } else if gui_info.hwndFocus.0 != 0 {
                            // 如果没有插入符，尝试使用焦点窗口的智能位置
                            let mut focus_rect = RECT {
                                left: 0,
                                top: 0,
                                right: 0,
                                bottom: 0,
                            };
                            if GetWindowRect(gui_info.hwndFocus, &mut focus_rect).is_ok() {
                                // 获取鼠标位置，如果在焦点窗口内则使用鼠标位置，否则使用窗口中心
                                let mut mouse_pos = POINT { x: 0, y: 0 };
                                if GetCursorPos(&mut mouse_pos).is_ok()
                                    && mouse_pos.x >= focus_rect.left
                                    && mouse_pos.x <= focus_rect.right
                                    && mouse_pos.y >= focus_rect.top
                                    && mouse_pos.y <= focus_rect.bottom
                                {
                                    caret_pos = mouse_pos;
                                    caret_source = "focus_window_mouse";
                                } else {
                                    // 使用窗口中心偏左上的位置
                                    caret_pos.x =
                                        focus_rect.left + (focus_rect.right - focus_rect.left) / 3;
                                    caret_pos.y =
                                        focus_rect.top + (focus_rect.bottom - focus_rect.top) / 3;
                                    caret_source = "focus_window_center";
                                }
                                use_caret = true;
                            }
                        }
                    }
                }
            }
        }

        // 如果无法获取插入符位置，回退到鼠标位置
        if !use_caret {
            if GetCursorPos(&mut caret_pos).is_err() {
                return Err("获取光标位置失败".to_string());
            }
            caret_source = "mouse";
        }

        // 获取插入符/鼠标所在的显示器信息
        let monitor = MonitorFromPoint(caret_pos, MONITOR_DEFAULTTONEAREST);
        let mut monitor_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            rcWork: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            dwFlags: 0,
        };

        let (
            screen_left,
            screen_top,
            screen_width,
            screen_height,
            work_left,
            work_top,
            work_width,
            work_height,
        ) = if GetMonitorInfoW(monitor, &mut monitor_info).as_bool() {
            let screen_left = monitor_info.rcMonitor.left;
            let screen_top = monitor_info.rcMonitor.top;
            let screen_width = monitor_info.rcMonitor.right - monitor_info.rcMonitor.left;
            let screen_height = monitor_info.rcMonitor.bottom - monitor_info.rcMonitor.top;

            let work_left = monitor_info.rcWork.left;
            let work_top = monitor_info.rcWork.top;
            let work_width = monitor_info.rcWork.right - monitor_info.rcWork.left;
            let work_height = monitor_info.rcWork.bottom - monitor_info.rcWork.top;

            (
                screen_left,
                screen_top,
                screen_width,
                screen_height,
                work_left,
                work_top,
                work_width,
                work_height,
            )
        } else {
            // 回退到主屏幕
            let screen_width = GetSystemMetrics(SM_CXSCREEN);
            let screen_height = GetSystemMetrics(SM_CYSCREEN);
            (
                0,
                0,
                screen_width,
                screen_height,
                0,
                0,
                screen_width,
                screen_height,
            )
        };

        // 获取窗口尺寸
        let window_size = window
            .outer_size()
            .map_err(|e| format!("获取窗口尺寸失败: {}", e))?;
        let window_width = window_size.width as i32;
        let window_height = window_size.height as i32;

        // 智能定位算法：优先在插入符/鼠标的左下角显示（使用工作区域）
        let margin = 12; // 边距
        let mut target_x;
        let mut target_y;
        let mut position_strategy = "unknown";

        // 策略1：尝试在左下角显示（优先策略）
        target_x = caret_pos.x - window_width - margin;
        target_y = caret_pos.y + margin;

        // 检查左下角是否在工作区域内有足够空间
        if target_x >= work_left
            && target_y + window_height <= work_top + work_height
            && target_x + window_width <= work_left + work_width
        {
            position_strategy = "left_bottom";
        } else {
            // 策略2：尝试在右下角显示
            target_x = caret_pos.x + margin;
            target_y = caret_pos.y + margin;

            if target_x >= work_left
                && target_x + window_width <= work_left + work_width
                && target_y + window_height <= work_top + work_height
            {
                position_strategy = "right_bottom";
            } else {
                // 策略3：尝试在左上角显示
                target_x = caret_pos.x - window_width - margin;
                target_y = caret_pos.y - window_height - margin;

                if target_x >= work_left
                    && target_y >= work_top
                    && target_x + window_width <= work_left + work_width
                {
                    position_strategy = "left_top";
                } else {
                    // 策略4：尝试在右上角显示
                    target_x = caret_pos.x + margin;
                    target_y = caret_pos.y - window_height - margin;

                    if target_x + window_width <= work_left + work_width && target_y >= work_top {
                        position_strategy = "right_top";
                    } else {
                        // 策略5：智能调整到最佳可用位置

                        // 水平方向：优先左侧，不够则右侧，再不够则居中
                        if caret_pos.x - window_width - margin >= work_left {
                            target_x = caret_pos.x - window_width - margin;
                        } else if caret_pos.x + margin + window_width <= work_left + work_width {
                            target_x = caret_pos.x + margin;
                        } else {
                            target_x = work_left + (work_width - window_width) / 2;
                        }

                        // 垂直方向：优先下方，不够则上方，再不够则居中
                        if caret_pos.y + margin + window_height <= work_top + work_height {
                            target_y = caret_pos.y + margin;
                        } else if caret_pos.y - window_height - margin >= work_top {
                            target_y = caret_pos.y - window_height - margin;
                        } else {
                            target_y = work_top + (work_height - window_height) / 2;
                        }

                        position_strategy = "adaptive";
                    }
                }
            }
        }

        // 最终边界检查和调整（确保在工作区域内）
        if target_x < work_left {
            target_x = work_left;
        } else if target_x + window_width > work_left + work_width {
            target_x = work_left + work_width - window_width;
        }

        if target_y < work_top {
            target_y = work_top;
        } else if target_y + window_height > work_top + work_height {
            target_y = work_top + work_height - window_height;
        }

        // 设置窗口位置
        let position = tauri::PhysicalPosition::new(target_x, target_y);
        window
            .set_position(position)
            .map_err(|e| format!("设置窗口位置失败: {}", e))?;

        Ok(())
    }
}

// 获取窗口类名
#[cfg(windows)]
fn get_window_class_name(hwnd: windows::Win32::Foundation::HWND) -> String {
    use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;

    let mut class_name: [u16; 256] = [0; 256];
    let len = unsafe { GetClassNameW(hwnd, &mut class_name) };

    if len > 0 {
        String::from_utf16_lossy(&class_name[..len as usize])
    } else {
        String::new()
    }
}

// 获取窗口标题
#[cfg(windows)]
fn get_window_title(hwnd: windows::Win32::Foundation::HWND) -> String {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowTextLengthW, GetWindowTextW};

    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len == 0 {
        return String::new();
    }

    let mut title: Vec<u16> = vec![0; (len + 1) as usize];
    let actual_len = unsafe { GetWindowTextW(hwnd, &mut title) };

    if actual_len > 0 {
        String::from_utf16_lossy(&title[..actual_len as usize])
    } else {
        String::new()
    }
}

// 识别是否为浏览器或WebView窗口
#[cfg(windows)]
fn is_browser_or_webview_window(class_name: &str, window_title: &str) -> bool {
    // 常见的浏览器和WebView窗口类名
    let browser_classes = [
        "Chrome_WidgetWin_1",     // Google Chrome
        "MozillaWindowClass",     // Firefox
        "ApplicationFrameWindow", // Microsoft Edge (UWP)
        "EdgeWebView",            // Microsoft Edge WebView
        "WebView2",               // WebView2
        "CefBrowserWindow",       // CEF (Chromium Embedded Framework)
        "WebKit",                 // WebKit
        "Electron",               // Electron
        "OperaWindowClass",       // Opera
        "BraveWindowClass",       // Brave
        "VivaldiWindowClass",     // Vivaldi
        "SafariWindow",           // Safari (if available on Windows)
        "QtWebEngineProcess",     // Qt WebEngine
        "WebBrowser",             // Generic WebBrowser
        "IEFrame",                // Internet Explorer
        "CabinetWClass",          // Windows Explorer (has web view)
    ];

    // 检查类名
    for browser_class in &browser_classes {
        if class_name.contains(browser_class) {
            return true;
        }
    }

    // 常见的浏览器窗口标题关键词
    let browser_title_keywords = [
        "Chrome",
        "Firefox",
        "Edge",
        "Opera",
        "Brave",
        "Vivaldi",
        "Safari",
        "Internet Explorer",
        "WebView",
        "Electron",
        "- Google Chrome",
        "- Mozilla Firefox",
        "- Microsoft Edge",
        "- Opera",
        "- Brave",
    ];

    // 检查窗口标题
    let title_lower = window_title.to_lowercase();
    for keyword in &browser_title_keywords {
        if title_lower.contains(&keyword.to_lowercase()) {
            return true;
        }
    }

    // 特殊检查：包含URL或网页相关内容的标题
    if title_lower.contains("http://")
        || title_lower.contains("https://")
        || title_lower.contains("www.")
        || title_lower.contains(".com")
        || title_lower.contains(".org")
        || title_lower.contains(".net")
        || title_lower.contains("localhost")
    {
        return true;
    }

    false
}

#[cfg(not(windows))]
pub fn position_window_at_cursor(_window: &WebviewWindow) -> Result<(), String> {
    // 非Windows平台暂不实现
    Ok(())
}
