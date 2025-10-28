// 跨平台粘贴工具函数

#[cfg(windows)]
pub fn windows_paste() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
        KEYEVENTF_KEYUP, VK_CONTROL, VK_V, VK_MENU,
    };

    unsafe {
        // 从 input_monitor 获取当前按键状态
        let (ctrl_pressed, alt_pressed, _, _) = crate::input_monitor::get_modifier_keys_state();

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
