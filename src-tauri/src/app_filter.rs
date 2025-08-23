use crate::settings;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

#[cfg(windows)]

#[cfg(windows)]
use windows::Win32::System::ProcessStatus::{GetModuleFileNameExW, GetProcessImageFileNameW};
#[cfg(windows)]
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub process: String,
    pub path: String,
    pub icon: Option<String>,
}

/// 获取所有可见窗口的信息
#[cfg(windows)]
pub fn get_all_windows_info() -> Vec<AppInfo> {
    use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
    use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;

    let mut windows = Vec::new();

    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let windows_ptr = lparam.0 as *mut Vec<AppInfo>;
        let windows = &mut *windows_ptr;

        // 只处理可见窗口
        if IsWindowVisible(hwnd).as_bool() {
            let mut process_id: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut process_id));

            if process_id > 0 {
                // 获取窗口标题
                let mut title_buffer = [0u16; 512];
                let title_len = GetWindowTextW(hwnd, &mut title_buffer);
                let window_title = if title_len > 0 {
                    String::from_utf16_lossy(&title_buffer[..title_len as usize])
                } else {
                    return BOOL(1); // 跳过无标题窗口
                };

                // 跳过空标题或系统窗口
                if window_title.trim().is_empty() || window_title == "Program Manager" {
                    return BOOL(1);
                }

                // 获取进程信息
                if let Ok(process_handle) = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, process_id) {
                    let mut buffer = [0u16; 260];
                    let len = GetModuleFileNameExW(process_handle, None, &mut buffer);
                    let full_path = if len > 0 {
                        String::from_utf16_lossy(&buffer[..len as usize])
                    } else {
                        String::from("unknown")
                    };

                    let process_filename = full_path
                        .split('\\')
                        .last()
                        .unwrap_or(&full_path)
                        .to_string();

                    // 获取应用图标
                    let icon = crate::file_handler::get_file_icon(&full_path).ok();

                    windows.push(AppInfo {
                        name: window_title,
                        process: process_filename,
                        path: full_path,
                        icon,
                    });
                }
            }
        }
        BOOL(1)
    }

    unsafe {
        EnumWindows(Some(enum_windows_proc), LPARAM(&mut windows as *mut _ as isize));
    }

    // 按窗口标题排序并去重
    windows.sort_by(|a: &AppInfo, b: &AppInfo| a.name.cmp(&b.name));
    windows.dedup_by(|a: &mut AppInfo, b: &mut AppInfo| a.process == b.process && a.name == b.name);
    windows
}

/// 检查当前应用是否在允许列表中
#[cfg(windows)]
pub fn is_current_app_allowed() -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};
    use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;

    let settings = settings::get_global_settings();
    
    // 如果未启用应用过滤，则允许所有应用
    if !settings.app_filter_enabled {
        return true;
    }

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == HWND(0) {
            return true; // 无法获取当前应用，默认允许
        }

        // 获取进程ID
        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));

        if process_id == 0 {
            return true;
        }

        // 获取进程路径
        let process_handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, process_id);
        let full_path = if let Ok(handle) = process_handle {
            let mut buffer = [0u16; 260];
            let len = GetModuleFileNameExW(handle, None, &mut buffer);
            if len > 0 {
                String::from_utf16_lossy(&buffer[..len as usize])
            } else {
                String::from("unknown")
            }
        } else {
            String::from("unknown")
        };

        // 获取窗口标题
        let mut title_buffer = [0u16; 512];
        let title_len = GetWindowTextW(hwnd, &mut title_buffer);
        let window_title = if title_len > 0 {
            String::from_utf16_lossy(&title_buffer[..title_len as usize])
        } else {
            String::from("unknown")
        };

        let process_filename = full_path
            .split('\\')
            .last()
            .unwrap_or(&full_path)
            .to_string();

        // 检查是否匹配任何过滤规则
        let matches_filter = settings.app_filter_list.iter().any(|filter| {
            let filter_lower = filter.to_lowercase();
            
            // 检查进程名
            if process_filename.to_lowercase().contains(&filter_lower) {
                return true;
            }
            
            // 检查窗口标题
            if window_title.to_lowercase().contains(&filter_lower) {
                return true;
            }
            
            // 检查完整路径
            if full_path.to_lowercase().contains(&filter_lower) {
                return true;
            }
            
            false
        });

        match settings.app_filter_mode.as_str() {
            "whitelist" => matches_filter, // 白名单模式：只有匹配的应用才允许
            "blacklist" => !matches_filter, // 黑名单模式：匹配的应用不允许
            _ => true, // 默认允许
        }
    }
}

/// 非Windows平台的占位实现
#[cfg(not(windows))]
pub fn is_current_app_allowed() -> bool {
    true
}



#[tauri::command]
pub fn get_all_windows_info_cmd() -> Result<Vec<AppInfo>, String> {
    #[cfg(windows)]
    {
        Ok(get_all_windows_info())
    }
    
    #[cfg(not(windows))]
    {
        Ok(Vec::new())
    }
}


/// 非Windows平台的占位实现
#[cfg(not(windows))]
pub fn get_active_window_process_name() -> Option<String> {
    None
}

#[cfg(not(windows))]
pub fn get_active_window_title() -> Option<String> {
    None
}

#[cfg(not(windows))]
pub fn is_current_app_allowed() -> bool {
    true
}