// 窗口相关工具函数

/// 检测当前活动窗口是否为文件管理器
#[cfg(windows)]
pub fn is_target_file_manager() -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == HWND(0) {
            return false;
        }

        // 获取窗口标题
        let mut window_title = [0u16; 256];
        let title_len = GetWindowTextW(hwnd, &mut window_title);
        if title_len > 0 {
            let title = String::from_utf16_lossy(&window_title[..title_len as usize]);

            // 通过窗口标题判断是否为文件管理器
            let title_lower = title.to_lowercase();
            let is_file_manager_by_title = title_lower.contains("文件资源管理器")
                || title_lower.contains("file explorer")
                || title_lower.contains("total commander")
                || title_lower.contains("freecommander")
                || title_lower.contains("directory opus")
                || title_lower.contains("q-dir")
                || title_lower.ends_with(" - 文件夹")
                || title_lower.ends_with(" - folder");

            return is_file_manager_by_title;
        }
    }

    false
}

#[cfg(not(windows))]
pub fn is_target_file_manager() -> bool {
    // 非Windows系统暂时返回false，不延迟
    false
}
