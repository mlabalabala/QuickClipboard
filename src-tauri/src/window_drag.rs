use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::WebviewWindow;
use windows::Win32::Foundation::{POINT, HWND};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, SetWindowPos, GetSystemMetrics,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    HWND_TOP, SWP_NOSIZE, SWP_NOZORDER
};
use windows::Win32::Graphics::Gdi::{MonitorFromWindow, GetMonitorInfoW, MONITORINFO, MONITOR_DEFAULTTONEAREST};

// 自定义拖拽状态
#[derive(Debug, Clone)]
pub struct CustomDragState {
    pub is_dragging: bool,
    pub window: WebviewWindow,
    pub mouse_offset_x: i32,
    pub mouse_offset_y: i32,
    pub hwnd: HWND,
}

// 全局拖拽状态管理
lazy_static::lazy_static! {
    static ref CUSTOM_DRAG_STATE: Arc<Mutex<Option<CustomDragState>>> = Arc::new(Mutex::new(None));
}

// 开始自定义拖拽
pub fn start_custom_drag(window: WebviewWindow, mouse_screen_x: i32, mouse_screen_y: i32) -> Result<(), String> {
    // 获取窗口句柄和位置信息
    let hwnd = HWND(window.hwnd().map_err(|e| format!("获取窗口句柄失败: {}", e))?.0 as isize);
    let physical_position = window.outer_position().map_err(|e| format!("获取窗口位置失败: {}", e))?;
    let scale_factor = window.scale_factor().map_err(|e| format!("获取缩放因子失败: {}", e))?;
    
    // 将前端逻辑坐标转换为物理坐标
    let mouse_physical_x = (mouse_screen_x as f64 * scale_factor) as i32;
    let mouse_physical_y = (mouse_screen_y as f64 * scale_factor) as i32;
    
    // 计算鼠标相对窗口的偏移（物理像素）
    let mouse_offset_x = mouse_physical_x - physical_position.x;
    let mouse_offset_y = mouse_physical_y - physical_position.y;
    
    // 保存拖拽状态
    {
        let mut drag_state = CUSTOM_DRAG_STATE.lock().map_err(|e| format!("锁定拖拽状态失败: {}", e))?;
        *drag_state = Some(CustomDragState {
            is_dragging: true,
            window: window.clone(),
            mouse_offset_x,
            mouse_offset_y,
            hwnd,
        });
    }
    
    // 启动拖拽监听线程
    start_drag_monitoring_thread();
    
    Ok(())
}

// 停止自定义拖拽
pub fn stop_custom_drag() -> Result<(), String> {
    // 停止拖拽
    let window = {
        let mut drag_state = CUSTOM_DRAG_STATE.lock().map_err(|e| format!("锁定拖拽状态失败: {}", e))?;
        if let Some(ref mut state) = drag_state.as_mut() {
            state.is_dragging = false;
            Some(state.window.clone())
        } else {
            None
        }
    };
    
    // 延迟检查边缘吸附（如果有窗口）
    if let Some(window) = window {
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(100));
            // 通知边缘吸附模块检查窗口位置
            let _ = crate::edge_snap::check_window_snap(&window);
        });
    }
    
    Ok(())
}

// 拖拽监听线程
fn start_drag_monitoring_thread() {
    std::thread::spawn(|| {
        loop {
            let (is_dragging, window, mouse_offset_x, mouse_offset_y, hwnd) = {
                match CUSTOM_DRAG_STATE.lock() {
                    Ok(state) => {
                        if let Some(ref drag_state) = state.as_ref() {
                            if !drag_state.is_dragging {
                                break;
                            }
                            (
                                drag_state.is_dragging,
                                drag_state.window.clone(),
                                drag_state.mouse_offset_x,
                                drag_state.mouse_offset_y,
                                drag_state.hwnd,
                            )
                        } else {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            };
            
            // 获取当前鼠标位置
            let mut cursor_pos = POINT::default();
            unsafe {
                if GetCursorPos(&mut cursor_pos).is_err() {
                    continue;
                }
            }
            
            // 计算新的窗口位置
            let new_physical_x = cursor_pos.x - mouse_offset_x;
            let new_physical_y = cursor_pos.y - mouse_offset_y;
            
            // 获取虚拟桌面边界并应用磁性吸附
            if let Ok(virtual_desktop) = get_virtual_screen_size() {
                let (virtual_x, virtual_y, virtual_width, virtual_height) = virtual_desktop;
                let (final_x, final_y) = apply_magnetic_snap_and_bounds(
                    new_physical_x, new_physical_y,
                    virtual_x, virtual_y, virtual_width, virtual_height,
                    &window
                );
                
                // 直接使用Win32 API设置窗口位置
                unsafe {
                    let _ = SetWindowPos(
                        hwnd,
                        HWND_TOP,
                        final_x,
                        final_y,
                        0,
                        0,
                        SWP_NOSIZE | SWP_NOZORDER,
                    );
                }
            }
            
            std::thread::sleep(Duration::from_micros(500));
        }
    });
}

// 获取虚拟桌面尺寸
pub fn get_virtual_screen_size() -> Result<(i32, i32, i32, i32), String> {
    #[cfg(windows)]
    {
        unsafe {
            let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
            Ok((x, y, w, h))
        }
    }
    #[cfg(not(windows))]
    {
        Ok((0, 0, 1920, 1080))
    }
}

// 获取窗口所在显示器的边界
#[cfg(windows)]
fn get_monitor_bounds_for_drag(window: &WebviewWindow) -> Result<(i32, i32, i32, i32), String> {
    use windows::Win32::Foundation::HWND;
    
    let hwnd = HWND(window.hwnd().map_err(|e| format!("获取窗口句柄失败: {}", e))?.0 as isize);
    
    unsafe {
        let hmonitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut monitor_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            rcMonitor: windows::Win32::Foundation::RECT::default(),
            rcWork: windows::Win32::Foundation::RECT::default(),
            dwFlags: 0,
        };
        
        if GetMonitorInfoW(hmonitor, &mut monitor_info).as_bool() {
            let monitor_rect = monitor_info.rcMonitor;
            Ok((
                monitor_rect.left,
                monitor_rect.top,
                monitor_rect.right - monitor_rect.left,
                monitor_rect.bottom - monitor_rect.top,
            ))
        } else {
            Err("获取显示器信息失败".to_string())
        }
    }
}

// 应用磁性吸附和边界限制
fn apply_magnetic_snap_and_bounds(
    mut x: i32, mut y: i32, 
    vx: i32, vy: i32, vw: i32, vh: i32,
    window: &WebviewWindow
) -> (i32, i32) {
    const MAGNETIC_DISTANCE: i32 = 40;
    
    if let Ok(window_size) = window.outer_size() {
        let pw = window_size.width as i32;
        let ph = window_size.height as i32;
        let monitor_bottom = get_monitor_bounds_for_drag(window).map(|(_, my, _, mh)| my + mh).unwrap_or(vy + vh);
        
        // 磁性吸附：左右上用虚拟桌面，下用当前显示器
        if (x - vx).abs() <= MAGNETIC_DISTANCE { x = vx; }
        else if ((vx + vw) - (x + pw)).abs() <= MAGNETIC_DISTANCE { x = vx + vw - pw; }
        
        if (y - vy).abs() <= MAGNETIC_DISTANCE { y = vy; }
        else if (monitor_bottom - (y + ph)).abs() <= MAGNETIC_DISTANCE { y = monitor_bottom - ph; }
        
        // 边界限制：只限制上下边界，允许跨屏拖拽
        if y + ph > vy + vh { y = vy + vh - ph; }
        if y < vy { y = vy; }
    }
    
    (x, y)
}

// 检查是否正在拖拽
pub fn is_dragging() -> bool {
    if let Ok(state) = CUSTOM_DRAG_STATE.lock() {
        state.as_ref().map_or(false, |s| s.is_dragging)
    } else {
        false
    }
}

// 获取当前拖拽的窗口
pub fn get_dragging_window() -> Option<WebviewWindow> {
    if let Ok(state) = CUSTOM_DRAG_STATE.lock() {
        state.as_ref().and_then(|s| if s.is_dragging { Some(s.window.clone()) } else { None })
    } else {
        None
    }
}

