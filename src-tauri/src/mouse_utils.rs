//鼠标位置工具模块


#[inline]
pub fn get_cursor_position() -> Result<(i32, i32), String> {
    crate::input_monitor::get_mouse_position()
}

#[cfg(windows)]
#[inline]
pub fn get_cursor_point() -> Result<windows::Win32::Foundation::POINT, String> {
    let (x, y) = get_cursor_position()?;
    Ok(windows::Win32::Foundation::POINT { x, y })
}

// 非 Windows 平台
#[cfg(not(windows))]
#[inline]
pub fn get_cursor_point() -> Result<(i32, i32), String> {
    get_cursor_position()
}

// 设置鼠标位置（物理像素坐标）
#[cfg(windows)]
#[inline]
pub fn set_cursor_position(x: i32, y: i32) -> Result<(), String> {
    use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;
    unsafe {
        SetCursorPos(x, y)
            .map_err(|e| format!("设置鼠标位置失败: {}", e))?;
        Ok(())
    }
}

// 非 Windows 平台：暂不支持
#[cfg(not(windows))]
#[inline]
pub fn set_cursor_position(_x: i32, _y: i32) -> Result<(), String> {
    Err("当前平台不支持设置鼠标位置".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cursor_position() {
        match get_cursor_position() {
            Ok((x, y)) => {
                println!("鼠标位置: ({}, {})", x, y);
            }
            Err(e) => {
                panic!("获取鼠标位置失败: {}", e);
            }
        }
    }

    #[test]
    fn test_get_cursor_point() {
        match get_cursor_point() {
            Ok(point) => {
                println!("鼠标位置: ({}, {})", point.x, point.y);
            }
            Err(e) => {
                panic!("获取鼠标位置失败: {}", e);
            }
        }
    }
}

