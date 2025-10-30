// 跨平台粘贴工具函数

#[cfg(windows)]
pub fn windows_paste() -> bool {
    // 引入了 VK_SHIFT 和 VK_INSERT
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
        KEYEVENTF_KEYUP, VK_SHIFT, VK_INSERT, VK_MENU,
    };

    unsafe {
        // 从 input_monitor 获取当前按键状态
        // 假设 get_modifier_keys_state 返回 (ctrl, alt, shift, win)
        let (_, alt_pressed, shift_pressed, _) = crate::input_monitor::get_modifier_keys_state();

        // 生成按键序列
        let mut inputs: Vec<INPUT> = Vec::new();

        // 如果Alt键被按下，先释放它以避免干扰粘贴操作
        if alt_pressed {
            let mut alt_up = INPUT::default();
            alt_up.r#type = INPUT_KEYBOARD;
            alt_up.Anonymous.ki = KEYBDINPUT {
                wVk: VK_MENU,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            inputs.push(alt_up);
        }

        // 如果 Shift 键没有被按下，则模拟按下 Shift
        if !shift_pressed {
            let mut shift_down = INPUT::default();
            shift_down.r#type = INPUT_KEYBOARD;
            shift_down.Anonymous.ki = KEYBDINPUT {
                wVk: VK_SHIFT, // 使用 VK_SHIFT
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: 0,
            };
            inputs.push(shift_down);
        }

        // 按下 Insert
        let mut insert_down = INPUT::default();
        insert_down.r#type = INPUT_KEYBOARD;
        insert_down.Anonymous.ki = KEYBDINPUT {
            wVk: VK_INSERT, // 使用 VK_INSERT
            wScan: 0,
            dwFlags: KEYBD_EVENT_FLAGS(0),
            time: 0,
            dwExtraInfo: 0,
        };
        inputs.push(insert_down);

        // 释放 Insert
        let mut insert_up = INPUT::default();
        insert_up.r#type = INPUT_KEYBOARD;
        insert_up.Anonymous.ki = KEYBDINPUT {
            wVk: VK_INSERT, // 使用 VK_INSERT
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };
        inputs.push(insert_up);

        // 如果 Shift 键之前不是由我们按下的，则模拟释放 Shift
        if !shift_pressed {
            let mut shift_up = INPUT::default();
            shift_up.r#type = INPUT_KEYBOARD;
            shift_up.Anonymous.ki = KEYBDINPUT {
                wVk: VK_SHIFT, // 使用 VK_SHIFT
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            inputs.push(shift_up);
        }

        // 如果Alt键之前被按下，恢复它的状态
        if alt_pressed {
            let mut alt_down = INPUT::default();
            alt_down.r#type = INPUT_KEYBOARD;
            alt_down.Anonymous.ki = KEYBDINPUT {
                wVk: VK_MENU,
                wScan: 0,
                dwFlags: KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: 0,
            };
            inputs.push(alt_down);
        }

        let result = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        result != 0
    }
}

// 非Windows平台的空实现
#[cfg(not(windows))]
pub fn windows_paste() -> bool {
    false
}
