use tauri::WebviewWindow;

#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, 
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN
};

#[derive(Clone, serde::Serialize, Debug)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

#[derive(Clone, serde::Serialize, Debug)]
pub struct CssMonitorInfo {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub is_primary: bool,
}

pub struct ScreenUtils;

impl ScreenUtils {
    pub fn physical_to_css(physical: f64, scale_factor: f64) -> f64 {
        physical / scale_factor
    }

    pub fn css_to_physical(css: f64, scale_factor: f64) -> f64 {
        css * scale_factor
    }

    pub fn get_css_monitors(window: &WebviewWindow) -> Result<Vec<CssMonitorInfo>, String> {
        let scale_factor = window.scale_factor().unwrap_or(1.0);
        let physical_monitors = Self::get_all_monitors_from_window(window)?;
        let css_monitors = physical_monitors.into_iter().map(|monitor| {
            CssMonitorInfo {
                x: Self::physical_to_css(monitor.x as f64, scale_factor),
                y: Self::physical_to_css(monitor.y as f64, scale_factor),
                width: Self::physical_to_css(monitor.width as f64, scale_factor),
                height: Self::physical_to_css(monitor.height as f64, scale_factor),
                is_primary: monitor.is_primary,
            }
        }).collect();
        Ok(css_monitors)
    }

    // 多显示器边界约束
    pub fn constrain_to_physical_bounds(
        x: i32, y: i32, width: i32, height: i32,
        window: &tauri::WebviewWindow
    ) -> Result<(i32, i32), String> {
        let monitors = Self::get_all_monitors_from_window(window)?;
        if monitors.is_empty() {
            return Ok((x.max(0), y.max(0)));
        }

        // 检查选区与哪些显示器有重叠
        let selection_right = x + width;
        let selection_bottom = y + height;
        
        let overlapping_monitors: Vec<_> = monitors.iter().filter(|monitor| {
            let monitor_right = monitor.x + monitor.width as i32;
            let monitor_bottom = monitor.y + monitor.height as i32;
            
            x < monitor_right && selection_right > monitor.x &&
            y < monitor_bottom && selection_bottom > monitor.y
        }).collect();

        if overlapping_monitors.len() > 1 {
            let (vx, vy, vw, vh) = Self::get_virtual_screen_size_from_window(window)?;
            let constrained_x = x.max(vx).min(vx + vw - width);
            let constrained_y = y.max(vy).min(vy + vh - height);
            Ok((constrained_x, constrained_y))
        } else if overlapping_monitors.len() == 1 {
            let monitor = overlapping_monitors[0];
            let monitor_right = monitor.x + (monitor.width as i32);
            let monitor_bottom = monitor.y + (monitor.height as i32);
            let constrained_x = x.max(monitor.x).min(monitor_right - width);
            let constrained_y = y.max(monitor.y).min(monitor_bottom - height);
            Ok((constrained_x, constrained_y))
        } else {
            let mut best_x = x;
            let mut best_y = y;
            let mut min_distance = i32::MAX as f64;

            for monitor in &monitors {
                let monitor_right = monitor.x + (monitor.width as i32);
                let monitor_bottom = monitor.y + (monitor.height as i32);
                let clamped_x = x.max(monitor.x).min(monitor_right - width);
                let clamped_y = y.max(monitor.y).min(monitor_bottom - height);
                let distance = ((clamped_x - x).pow(2) + (clamped_y - y).pow(2)) as f64;

                if distance < min_distance {
                    min_distance = distance;
                    best_x = clamped_x;
                    best_y = clamped_y;
                }
            }
            Ok((best_x, best_y))
        }
    }

    pub fn get_all_monitors_from_window(window: &WebviewWindow) -> Result<Vec<MonitorInfo>, String> {
        let monitors = window
            .available_monitors()
            .map_err(|e| format!("获取显示器列表失败: {}", e))?;
        
        let primary_monitor = window.primary_monitor()
            .map_err(|e| format!("获取主显示器失败: {}", e))?;
        
        let primary_name = primary_monitor.as_ref().and_then(|m| m.name());
        
        let result: Vec<MonitorInfo> = monitors
            .iter()
            .map(|monitor| {
                let position = monitor.position();
                let size = monitor.size();
                let is_primary = monitor.name() == primary_name;
                
                MonitorInfo {
                    x: position.x,
                    y: position.y,
                    width: size.width,
                    height: size.height,
                    is_primary,
                }
            })
            .collect();
        
        Ok(result)
    }
    
    pub fn get_virtual_screen_size_from_window(window: &WebviewWindow) -> Result<(i32, i32, i32, i32), String> {
        let monitors = Self::get_all_monitors_from_window(window)?;
        
        if monitors.is_empty() {
            return Err("没有找到显示器".to_string());
        }
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        
        for monitor in &monitors {
            min_x = min_x.min(monitor.x);
            min_y = min_y.min(monitor.y);
            max_x = max_x.max(monitor.x + monitor.width as i32);
            max_y = max_y.max(monitor.y + monitor.height as i32);
        }
        
        let width = max_x - min_x;
        let height = max_y - min_y;
        
        Ok((min_x, min_y, width, height))
    }

    // 用于无窗口上下文的场景（edge_snap、window_drag）
    pub fn get_virtual_screen_size() -> Result<(i32, i32, i32, i32), String> {
        #[cfg(windows)]
        {
            unsafe {
                let x = GetSystemMetrics(SM_XVIRTUALSCREEN);
                let y = GetSystemMetrics(SM_YVIRTUALSCREEN);
                let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
                let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
                Ok((x, y, width, height))
            }
        }
        #[cfg(not(windows))]
        {
            Ok((0, 0, 1920, 1080))
        }
    }

    pub fn get_monitor_bounds(window: &WebviewWindow) -> Result<(i32, i32, i32, i32), String> {
        let monitor = window
            .current_monitor()
            .map_err(|e| format!("获取当前显示器失败: {}", e))?
            .ok_or_else(|| "当前显示器不存在".to_string())?;
        
        let position = monitor.position();
        let size = monitor.size();
        
        Ok((position.x, position.y, size.width as i32, size.height as i32))
    }
}
