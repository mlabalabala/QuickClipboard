use std::time::Duration;
use tokio::time::sleep;

/// 文本输入模拟器配置
#[derive(Debug, Clone)]
pub struct InputSimulatorConfig {
    /// 输入速度（字符/秒）
    pub chars_per_second: u32,
    /// 是否逐字符输入（false为逐词输入）
    pub char_by_char: bool,
    /// 输入间隔的随机变化范围（毫秒）
    pub jitter_ms: u32,
}

impl Default for InputSimulatorConfig {
    fn default() -> Self {
        Self {
            chars_per_second: 50,
            char_by_char: true,
            jitter_ms: 10,
        }
    }
}

/// 文本输入模拟器
pub struct TextInputSimulator {
    config: InputSimulatorConfig,
}

impl TextInputSimulator {
    /// 创建新的文本输入模拟器
    pub fn new(config: InputSimulatorConfig) -> Self {
        Self { config }
    }

    /// 模拟输入文本（流式）
    pub async fn simulate_input(&self, text: &str) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }

        if self.config.char_by_char {
            self.simulate_char_by_char(text).await
        } else {
            self.simulate_word_by_word(text).await
        }
    }

    /// 逐字符输入
    async fn simulate_char_by_char(&self, text: &str) -> Result<(), String> {
        let chars: Vec<char> = text.chars().collect();
        let delay_ms = 1000 / self.config.chars_per_second as u64;

        for ch in chars {
            self.send_unicode_char(ch)?;

            // 计算延迟时间（加入随机抖动）
            let jitter = if self.config.jitter_ms > 0 {
                fastrand::u32(0..=self.config.jitter_ms) as i32 - (self.config.jitter_ms as i32 / 2)
            } else {
                0
            };

            let actual_delay = (delay_ms as i32 + jitter).max(1) as u64;
            sleep(Duration::from_millis(actual_delay)).await;
        }

        Ok(())
    }

    /// 逐词输入
    async fn simulate_word_by_word(&self, text: &str) -> Result<(), String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let chars_per_word = text.len() / words.len().max(1);
        let delay_ms = (chars_per_word as u64 * 1000) / self.config.chars_per_second as u64;

        for (i, word) in words.iter().enumerate() {
            // 输入单词
            for ch in word.chars() {
                self.send_unicode_char(ch)?;
            }

            // 如果不是最后一个词，添加空格
            if i < words.len() - 1 {
                self.send_unicode_char(' ')?;
            }

            // 延迟
            if i < words.len() - 1 {
                let jitter = if self.config.jitter_ms > 0 {
                    fastrand::u32(0..=self.config.jitter_ms) as i32
                        - (self.config.jitter_ms as i32 / 2)
                } else {
                    0
                };

                let actual_delay = (delay_ms as i32 + jitter).max(1) as u64;
                sleep(Duration::from_millis(actual_delay)).await;
            }
        }

        Ok(())
    }

    /// 发送Unicode字符
    #[cfg(windows)]
    fn send_unicode_char(&self, ch: char) -> Result<(), String> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
            VK_RETURN, VK_SHIFT,
        };

        // 特殊处理换行符：发送Shift+Enter组合键而不是Unicode换行字符
        if ch == '\n' {
            return self.send_key_combination(&[VK_SHIFT.0 as u16, VK_RETURN.0 as u16]);
        }

        // 处理Unicode代理对（surrogate pairs）
        let mut utf16_buffer = [0u16; 2];
        let utf16_slice = ch.encode_utf16(&mut utf16_buffer);
        let slice_len = utf16_slice.len();

        for unicode_value in utf16_slice {
            // 按下键
            let mut input_down = INPUT::default();
            input_down.r#type = INPUT_KEYBOARD;
            input_down.Anonymous.ki = KEYBDINPUT {
                wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0),
                wScan: *unicode_value,
                dwFlags: KEYEVENTF_UNICODE,
                time: 0,
                dwExtraInfo: 0,
            };

            // 释放键
            let mut input_up = INPUT::default();
            input_up.r#type = INPUT_KEYBOARD;
            input_up.Anonymous.ki = KEYBDINPUT {
                wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0),
                wScan: *unicode_value,
                dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };

            let inputs = [input_down, input_up];

            unsafe {
                let result = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                if result != inputs.len() as u32 {
                    return Err(format!(
                        "发送Unicode字符'{}'失败，期望发送{}个事件，实际发送{}个",
                        ch,
                        inputs.len(),
                        result
                    ));
                }
            }

            // 在代理对之间添加小延迟
            if slice_len > 1 {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }

        Ok(())
    }

    /// 发送虚拟键码
    #[cfg(windows)]
    fn send_virtual_key(&self, vk_code: u16) -> Result<(), String> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
        };

        // 按下键
        let mut input_down = INPUT::default();
        input_down.r#type = INPUT_KEYBOARD;
        input_down.Anonymous.ki = KEYBDINPUT {
            wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk_code),
            wScan: 0,
            dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0),
            time: 0,
            dwExtraInfo: 0,
        };

        // 释放键
        let mut input_up = INPUT::default();
        input_up.r#type = INPUT_KEYBOARD;
        input_up.Anonymous.ki = KEYBDINPUT {
            wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(vk_code),
            wScan: 0,
            dwFlags: KEYEVENTF_KEYUP,
            time: 0,
            dwExtraInfo: 0,
        };

        let inputs = [input_down, input_up];

        unsafe {
            let result = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            if result != inputs.len() as u32 {
                return Err(format!(
                    "发送虚拟键码{}失败，期望发送{}个事件，实际发送{}个",
                    vk_code,
                    inputs.len(),
                    result
                ));
            }
        }

        Ok(())
    }

    /// 发送组合键
    #[cfg(windows)]
    fn send_key_combination(&self, keys: &[u16]) -> Result<(), String> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
        };

        if keys.is_empty() {
            return Ok(());
        }

        let mut inputs = Vec::new();

        // 按下所有键（按顺序）
        for &key in keys {
            let mut input_down = INPUT::default();
            input_down.r#type = INPUT_KEYBOARD;
            input_down.Anonymous.ki = KEYBDINPUT {
                wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(key),
                wScan: 0,
                dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: 0,
            };
            inputs.push(input_down);
        }

        // 释放所有键（逆序）
        for &key in keys.iter().rev() {
            let mut input_up = INPUT::default();
            input_up.r#type = INPUT_KEYBOARD;
            input_up.Anonymous.ki = KEYBDINPUT {
                wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(key),
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            };
            inputs.push(input_up);
        }

        unsafe {
            let result = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
            if result != inputs.len() as u32 {
                return Err(format!(
                    "发送组合键失败，期望发送{}个事件，实际发送{}个",
                    inputs.len(),
                    result
                ));
            }
        }

        Ok(())
    }

    /// 发送Unicode字符（非Windows平台的占位实现）
    #[cfg(not(windows))]
    fn send_unicode_char(&self, _ch: char) -> Result<(), String> {
        Err("文本输入模拟仅支持Windows平台".to_string())
    }

    /// 更新配置
    pub fn update_config(&mut self, config: InputSimulatorConfig) {
        self.config = config;
    }

    /// 获取当前配置
    pub fn get_config(&self) -> &InputSimulatorConfig {
        &self.config
    }

    /// 立即输入文本（不使用流式效果）
    pub fn input_text_immediately(&self, text: &str) -> Result<(), String> {
        for ch in text.chars() {
            self.send_unicode_char(ch)?;
        }
        Ok(())
    }
}

/// 从应用设置创建输入模拟器配置
pub fn config_from_settings(settings: &crate::settings::AppSettings) -> InputSimulatorConfig {
    InputSimulatorConfig {
        chars_per_second: settings.ai_input_speed,
        char_by_char: true, // 默认逐字符输入
        jitter_ms: 10,      // 默认10ms抖动
    }
}

/// 全局文本输入模拟器实例
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};

static GLOBAL_INPUT_SIMULATOR: Lazy<Arc<Mutex<TextInputSimulator>>> = Lazy::new(|| {
    let config = InputSimulatorConfig::default();
    Arc::new(Mutex::new(TextInputSimulator::new(config)))
});

/// 获取全局输入模拟器
pub fn get_global_input_simulator() -> Arc<Mutex<TextInputSimulator>> {
    GLOBAL_INPUT_SIMULATOR.clone()
}

/// 更新全局输入模拟器配置
pub fn update_global_input_simulator_config(config: InputSimulatorConfig) {
    if let Ok(mut simulator) = GLOBAL_INPUT_SIMULATOR.lock() {
        simulator.update_config(config);
    }
}

/// 使用全局模拟器输入文本（流式）
pub async fn simulate_text_input(text: &str) -> Result<(), String> {
    let simulator = get_global_input_simulator();
    let result = if let Ok(sim) = simulator.lock() {
        sim.simulate_input(text).await
    } else {
        Err("无法获取输入模拟器锁".to_string())
    };
    result
}

/// 流式输入文本片段（用于AI翻译的实时输出）
pub async fn simulate_text_chunk_input(chunk: &str) -> Result<(), String> {
    if chunk.is_empty() {
        return Ok(());
    }

    let simulator = get_global_input_simulator();
    let chars: Vec<char> = chunk.chars().collect();

    // 先获取配置，然后释放锁
    let (delay_ms, jitter_ms) = {
        let sim = simulator
            .lock()
            .map_err(|_| "无法获取输入模拟器锁".to_string())?;
        let config = sim.get_config();
        let delay_ms = if config.chars_per_second > 0 {
            1000 / config.chars_per_second as u64
        } else {
            20 // 默认延迟
        };
        (delay_ms, config.jitter_ms)
    }; // 锁在这里被释放

    // 对于流式输入，我们逐字符输入以获得更好的视觉效果
    for (i, ch) in chars.iter().enumerate() {
        // 每次输入字符时重新获取锁
        {
            let sim = simulator
                .lock()
                .map_err(|_| "无法获取输入模拟器锁".to_string())?;
            sim.send_unicode_char(*ch)?;
        } // 锁在这里被释放

        // 除了最后一个字符，都添加延迟
        if i < chars.len() - 1 {
            // 添加小的随机抖动以模拟自然输入
            let jitter = if jitter_ms > 0 {
                fastrand::u32(0..=jitter_ms) as i32 - (jitter_ms as i32 / 2)
            } else {
                0
            };

            let actual_delay = (delay_ms as i32 + jitter).max(1) as u64;
            tokio::time::sleep(tokio::time::Duration::from_millis(actual_delay)).await;
        }
    }

    Ok(())
}

/// 使用全局模拟器立即输入文本
pub fn input_text_immediately(text: &str) -> Result<(), String> {
    let simulator = get_global_input_simulator();
    let result = if let Ok(sim) = simulator.lock() {
        sim.input_text_immediately(text)
    } else {
        Err("无法获取输入模拟器锁".to_string())
    };
    result
}
