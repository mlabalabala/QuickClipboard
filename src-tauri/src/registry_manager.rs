// Windows注册表管理器 - 用于禁用/启用系统Win组合键

#[cfg(windows)]
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
};

// 注册表路径
#[cfg(windows)]
const EXPLORER_ADVANCED_PATH: &str = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced";
#[cfg(windows)]
const DISABLED_HOTKEYS_VALUE: &str = "DisabledHotkeys";

// 禁用Windows系统Win+V快捷键（并重启Explorer）
#[cfg(windows)]
pub fn disable_win_v_hotkey() -> Result<(), String> {
    add_disabled_hotkey('V', true)
}

// 禁用Windows系统Win+V快捷键（静默模式，不重启Explorer）
#[cfg(windows)]
pub fn disable_win_v_hotkey_silent() -> Result<(), String> {
    add_disabled_hotkey('V', false)
}

// 启用Windows系统Win+V快捷键（并重启Explorer）
#[cfg(windows)]
pub fn enable_win_v_hotkey() -> Result<(), String> {
    remove_disabled_hotkey('V', true)
}

// 启用Windows系统Win+V快捷键（静默模式，不重启Explorer）
#[cfg(windows)]
pub fn enable_win_v_hotkey_silent() -> Result<(), String> {
    remove_disabled_hotkey('V', false)
}

// 添加禁用的快捷键
#[cfg(windows)]
fn add_disabled_hotkey(key: char, restart_explorer: bool) -> Result<(), String> {
    unsafe {
        let path: Vec<u16> = EXPLORER_ADVANCED_PATH
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        
        let value_name: Vec<u16> = DISABLED_HOTKEYS_VALUE
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let mut hkey: HKEY = HKEY::default();

        // 打开或创建注册表项
        let result = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(path.as_ptr()),
            0,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_READ | KEY_WRITE,
            None,
            &mut hkey,
            None,
        );

        if result.is_err() {
            return Err(format!("无法打开注册表项: {:?}", result));
        }

        // 读取当前值
        let current_value = read_disabled_hotkeys_value(hkey);
        
        // 添加新的键（如果还不存在）
        let key_upper = key.to_uppercase().to_string();
        let new_value = if current_value.contains(&key_upper) {
            current_value
        } else {
            format!("{}{}", current_value, key_upper)
        };

        // 转换为UTF-16并添加null终止符
        let data: Vec<u16> = new_value
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        // 设置新值
        let set_result = RegSetValueExW(
            hkey,
            windows::core::PCWSTR(value_name.as_ptr()),
            0,
            REG_SZ,
            Some(std::slice::from_raw_parts(
                data.as_ptr() as *const u8,
                data.len() * 2,
            )),
        );

        let _ = RegCloseKey(hkey);

        if set_result.is_err() {
            return Err(format!("无法设置注册表值: {:?}", set_result));
        }

        println!("已禁用Win+{}快捷键", key_upper);
        
        // 根据参数决定是否重启Explorer
        if restart_explorer {
            restart_explorer_process()?;
        } else {
            println!("注册表已修改，需要重启Explorer或重新登录后生效");
        }
        
        Ok(())
    }
}

// 移除禁用的快捷键
#[cfg(windows)]
fn remove_disabled_hotkey(key: char, restart_explorer: bool) -> Result<(), String> {
    unsafe {
        let path: Vec<u16> = EXPLORER_ADVANCED_PATH
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        
        let value_name: Vec<u16> = DISABLED_HOTKEYS_VALUE
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let mut hkey: HKEY = HKEY::default();

        // 打开注册表项
        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(path.as_ptr()),
            0,
            KEY_READ | KEY_WRITE,
            &mut hkey,
        );

        if result.is_err() {
            // 如果注册表项不存在，说明没有禁用任何快捷键
            return Ok(());
        }

        // 读取当前值
        let current_value = read_disabled_hotkeys_value(hkey);
        
        // 移除指定的键
        let key_upper = key.to_uppercase().to_string();
        let new_value = current_value.replace(&key_upper, "");

        if new_value.is_empty() {
            // 如果没有剩余的禁用键，删除整个值
            let delete_result = RegDeleteValueW(
                hkey,
                windows::core::PCWSTR(value_name.as_ptr()),
            );
            let _ = RegCloseKey(hkey);
            
            if delete_result.is_ok() {
                println!("已删除DisabledHotkeys注册表值");
            }
        } else {
            // 更新值
            let data: Vec<u16> = new_value
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let set_result = RegSetValueExW(
                hkey,
                windows::core::PCWSTR(value_name.as_ptr()),
                0,
                REG_SZ,
                Some(std::slice::from_raw_parts(
                    data.as_ptr() as *const u8,
                    data.len() * 2,
                )),
            );

            let _ = RegCloseKey(hkey);

            if set_result.is_err() {
                return Err(format!("无法更新注册表值: {:?}", set_result));
            }
        }

        println!("已启用Win+{}快捷键", key_upper);
        
        // 根据参数决定是否重启Explorer
        if restart_explorer {
            restart_explorer_process()?;
        } else {
            println!("注册表已修改，需要重启Explorer或重新登录后生效");
        }
        
        Ok(())
    }
}

// 读取DisabledHotkeys注册表值
#[cfg(windows)]
unsafe fn read_disabled_hotkeys_value(hkey: HKEY) -> String {
    let value_name: Vec<u16> = DISABLED_HOTKEYS_VALUE
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut buffer: Vec<u16> = vec![0; 256];
    let mut buffer_size: u32 = (buffer.len() * 2) as u32;

    let result = RegQueryValueExW(
        hkey,
        windows::core::PCWSTR(value_name.as_ptr()),
        None,
        None,
        Some(buffer.as_mut_ptr() as *mut u8),
        Some(&mut buffer_size),
    );

    if result.is_ok() && buffer_size > 0 {
        let len = (buffer_size as usize / 2).saturating_sub(1);
        String::from_utf16_lossy(&buffer[..len])
    } else {
        String::new()
    }
}

// 重启Explorer进程以使注册表更改生效
#[cfg(windows)]
fn restart_explorer_process() -> Result<(), String> {
    use std::process::Command;
    
    println!("正在重启Explorer进程以应用更改...");
    
    // 结束Explorer进程
    let kill_result = Command::new("taskkill")
        .args(&["/F", "/IM", "explorer.exe"])
        .output();
    
    if let Err(e) = kill_result {
        return Err(format!("无法结束Explorer进程: {}", e));
    }
    
    // 等待Explorer完全退出
    println!("等待Explorer退出...");
    std::thread::sleep(std::time::Duration::from_millis(1000));
    
    // 启动Explorer进程
    println!("启动Explorer...");
    let start_result = Command::new("cmd")
        .args(&["/C", "start", "explorer.exe"])
        .spawn();
    
    if let Err(e) = start_result {
        // 如果启动失败，尝试直接启动
        println!("使用cmd启动失败，尝试直接启动...");
        let fallback_result = Command::new("explorer.exe").spawn();
        if let Err(e2) = fallback_result {
            return Err(format!("无法启动Explorer进程: {} / {}", e, e2));
        }
    }
    
    // 等待Explorer启动完成
    std::thread::sleep(std::time::Duration::from_millis(1000));
    
    println!("Explorer进程已重启");
    Ok(())
}

// 检查Win+V快捷键是否被禁用
#[cfg(windows)]
pub fn is_win_v_hotkey_disabled() -> bool {
    is_hotkey_disabled('V')
}

// 检查指定快捷键是否被禁用
#[cfg(windows)]
fn is_hotkey_disabled(key: char) -> bool {
    unsafe {
        let path: Vec<u16> = EXPLORER_ADVANCED_PATH
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let mut hkey: HKEY = HKEY::default();

        let result = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            windows::core::PCWSTR(path.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        );

        if result.is_err() {
            return false;
        }

        let current_value = read_disabled_hotkeys_value(hkey);
        let _ = RegCloseKey(hkey);

        let key_upper = key.to_uppercase().to_string();
        current_value.contains(&key_upper)
    }
}

// 非Windows平台的空实现
#[cfg(not(windows))]
pub fn disable_win_v_hotkey() -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn disable_win_v_hotkey_silent() -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn enable_win_v_hotkey() -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn enable_win_v_hotkey_silent() -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn is_win_v_hotkey_disabled() -> bool {
    false
}

