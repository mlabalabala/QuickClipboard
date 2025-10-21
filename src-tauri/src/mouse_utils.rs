//鼠标位置工具模块

use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

#[inline]
pub fn get_cursor_position() -> Result<(i32, i32), String> {
    unsafe {
        let mut cursor_pos = POINT { x: 0, y: 0 };
        GetCursorPos(&mut cursor_pos)
            .map_err(|e| format!("获取鼠标位置失败: {}", e))?;
        Ok((cursor_pos.x, cursor_pos.y))
    }
}

#[inline]
pub fn get_cursor_point() -> Result<POINT, String> {
    unsafe {
        let mut cursor_pos = POINT { x: 0, y: 0 };
        GetCursorPos(&mut cursor_pos)
            .map_err(|e| format!("获取鼠标位置失败: {}", e))?;
        Ok(cursor_pos)
    }
}

/// 设置鼠标位置（物理像素坐标）
#[inline]
pub fn set_cursor_position(x: i32, y: i32) -> Result<(), String> {
    unsafe {
        SetCursorPos(x, y)
            .map_err(|e| format!("设置鼠标位置失败: {}", e))?;
        Ok(())
    }
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

