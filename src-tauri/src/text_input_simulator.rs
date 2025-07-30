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
    /// 是否保持原始格式（换行符、制表符等）
    pub preserve_formatting: bool,
    /// 换行符处理模式
    pub newline_mode: NewlineMode,
}

/// 换行符处理模式
#[derive(Debug, Clone)]
pub enum NewlineMode {
    /// 直接发送Unicode换行字符
    Unicode,
    /// 发送Enter键
    Enter,
    /// 发送Shift+Enter组合键
    ShiftEnter,
    /// 根据上下文自动选择
    Auto,
}

impl Default for InputSimulatorConfig {
    fn default() -> Self {
        Self {
            chars_per_second: 50,
            char_by_char: true,
            jitter_ms: 10,
            preserve_formatting: true,
            newline_mode: NewlineMode::Auto,
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
        self.send_unicode_char_with_retry(ch, 3)
    }

    /// 发送Unicode字符（带重试机制）
    #[cfg(windows)]
    fn send_unicode_char_with_retry(&self, ch: char, max_retries: u32) -> Result<(), String> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
            VK_RETURN, VK_SHIFT, VK_TAB,
        };

        // 调试关键字符输入（数字、冒号等）
        if ch.is_ascii_digit() || ch == ':' || ch == '：' {
            println!("输入关键字符: '{}' (U+{:04X})", ch, ch as u32);
        }

        // 根据配置处理特殊字符
        if self.config.preserve_formatting {
            match ch {
                '\n' => {
                    return self.handle_newline_char(max_retries);
                }
                '\r' => {
                    // 回车符：发送Enter键
                    return self.send_virtual_key(VK_RETURN.0 as u16);
                }
                '\t' => {
                    // 制表符：根据配置选择处理方式
                    return self.handle_tab_char(max_retries);
                }
                // 其他空白字符的特殊处理
                '\u{00A0}' => {
                    // 不间断空格 (Non-breaking space)
                    return self.send_unicode_char_raw(ch, max_retries);
                }
                '\u{2000}'..='\u{200F}' | '\u{2028}'..='\u{202F}' => {
                    // Unicode空白字符和格式字符
                    return self.send_unicode_char_raw(ch, max_retries);
                }
                _ => {}
            }
        }

        // 处理普通Unicode字符
        let result = self.send_unicode_char_raw(ch, max_retries);

        // 如果是关键字符，检查发送结果并尝试降级处理
        if ch.is_ascii_digit() || ch == ':' || ch == '：' {
            match &result {
                Ok(()) => {
                    println!("关键字符 '{}' 发送成功", ch);
                }
                Err(e) => {
                    println!("关键字符 '{}' Unicode发送失败: {}", ch, e);
                    // 尝试使用虚拟键码发送ASCII字符
                    if ch.is_ascii() {
                        println!("尝试使用虚拟键码发送ASCII字符 '{}'", ch);
                        return self.send_ascii_char_as_virtual_key(ch);
                    }
                }
            }
        }

        result
    }

    /// 处理换行符
    #[cfg(windows)]
    fn handle_newline_char(&self, max_retries: u32) -> Result<(), String> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{VK_RETURN, VK_SHIFT};

        println!("处理换行符: 模式={:?}", self.config.newline_mode);

        let result = match self.config.newline_mode {
            NewlineMode::Unicode => {
                println!("尝试Unicode换行字符");
                // 直接发送Unicode换行字符
                self.send_unicode_char_raw('\n', max_retries)
            }
            NewlineMode::Enter => {
                println!("发送Enter键");
                // 发送Enter键
                self.send_virtual_key(VK_RETURN.0 as u16)
            }
            NewlineMode::ShiftEnter => {
                println!("发送Shift+Enter组合键");
                // 发送Shift+Enter组合键
                self.send_key_combination(&[VK_SHIFT.0 as u16, VK_RETURN.0 as u16])
            }
            NewlineMode::Auto => {
                println!("自动模式：尝试Enter键（更兼容）");
                // 修改自动模式：优先使用Enter键，因为它更兼容
                match self.send_virtual_key(VK_RETURN.0 as u16) {
                    Ok(()) => {
                        println!("Enter键发送成功");

                        // 换行后添加额外延迟，让目标应用程序稳定
                        std::thread::sleep(std::time::Duration::from_millis(100));

                        // 尝试确保焦点仍在目标窗口
                        self.ensure_target_window_focus();

                        Ok(())
                    }
                    Err(e) => {
                        println!("Enter键失败，尝试Unicode: {}", e);
                        self.send_unicode_char_raw('\n', max_retries)
                    }
                }
            }
        };

        match &result {
            Ok(()) => println!("换行符处理成功"),
            Err(e) => println!("换行符处理失败: {}", e),
        }

        result
    }

    /// 确保目标窗口保持焦点
    #[cfg(windows)]
    fn ensure_target_window_focus(&self) {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, SetForegroundWindow};

        unsafe {
            let current_window = GetForegroundWindow();
            if current_window.0 != 0 {
                // 重新设置前台窗口以确保焦点
                let _ = SetForegroundWindow(current_window);
                println!("重新确保窗口焦点");
            }
        }
    }

    /// 处理制表符
    #[cfg(windows)]
    fn handle_tab_char(&self, max_retries: u32) -> Result<(), String> {
        use windows::Win32::UI::Input::KeyboardAndMouse::VK_TAB;

        // 优先尝试发送Tab键，失败时使用Unicode字符
        match self.send_virtual_key(VK_TAB.0 as u16) {
            Ok(()) => Ok(()),
            Err(_) => {
                // 降级到Unicode制表符
                self.send_unicode_char_raw('\t', max_retries)
            }
        }
    }

    /// 使用虚拟键码发送ASCII字符（降级方法）
    #[cfg(windows)]
    fn send_ascii_char_as_virtual_key(&self, ch: char) -> Result<(), String> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_0, VK_1,
            VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8, VK_9, VK_SHIFT,
        };

        if !ch.is_ascii() {
            return Err(format!("字符 '{}' 不是ASCII字符", ch));
        }

        let virtual_key = match ch {
            '0' => VK_0,
            '1' => VK_1,
            '2' => VK_2,
            '3' => VK_3,
            '4' => VK_4,
            '5' => VK_5,
            '6' => VK_6,
            '7' => VK_7,
            '8' => VK_8,
            '9' => VK_9,
            ':' => {
                // 冒号需要Shift+分号键
                return self.send_key_combination(&[VK_SHIFT.0 as u16, 0xBA]); // 0xBA是分号键
            }
            _ => {
                return Err(format!("不支持的ASCII字符: '{}'", ch));
            }
        };

        println!("使用虚拟键码发送字符 '{}' (VK={:?})", ch, virtual_key);
        self.send_virtual_key(virtual_key.0 as u16)
    }

    /// 发送原始Unicode字符
    #[cfg(windows)]
    fn send_unicode_char_raw(&self, ch: char, max_retries: u32) -> Result<(), String> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
        };

        for attempt in 0..max_retries {
            // 处理Unicode代理对（surrogate pairs）
            let mut utf16_buffer = [0u16; 2];
            let utf16_slice = ch.encode_utf16(&mut utf16_buffer);

            // 将UTF-16值复制到Vec中以避免借用问题
            let utf16_values: Vec<u16> = utf16_slice.iter().copied().collect();

            // 为代理对创建所有输入事件
            let mut inputs = Vec::new();

            // 按下所有代理对部分
            for unicode_value in &utf16_values {
                let input_down = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0),
                            wScan: *unicode_value,
                            dwFlags: KEYEVENTF_UNICODE,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                inputs.push(input_down);
            }

            // 释放所有代理对部分（逆序）
            for unicode_value in utf16_values.iter().rev() {
                let input_up = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0),
                            wScan: *unicode_value,
                            dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                inputs.push(input_up);
            }

            // 原子性发送所有输入事件
            unsafe {
                let result = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                if result == inputs.len() as u32 {
                    return Ok(()); // 成功
                }

                // 如果不是最后一次尝试，等待后重试
                if attempt < max_retries - 1 {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }

                return Err(format!(
                    "发送Unicode字符'{}'失败，尝试{}次后仍然失败。期望发送{}个事件，实际发送{}个",
                    ch,
                    max_retries,
                    inputs.len(),
                    result
                ));
            }
        }

        Err(format!("发送Unicode字符'{}'失败，超过最大重试次数", ch))
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
    // 根据设置字符串转换为换行符模式
    let newline_mode = match settings.ai_newline_mode.as_str() {
        "unicode" => NewlineMode::Unicode,
        "enter" => NewlineMode::Enter,
        "shift_enter" => NewlineMode::ShiftEnter,
        "auto" => NewlineMode::Auto,
        _ => NewlineMode::ShiftEnter, // 默认使用Shift+Enter
    };

    InputSimulatorConfig {
        chars_per_second: settings.ai_input_speed,
        char_by_char: true,        // 默认逐字符输入
        jitter_ms: 10,             // 默认10ms抖动
        preserve_formatting: true, // 默认保持格式
        newline_mode,              // 根据设置配置换行符模式
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

    // 批量输入方法
    simulate_text_chunk_input_batched(chunk).await
}

/// 批量流式输入文本片段
pub async fn simulate_text_chunk_input_batched(chunk: &str) -> Result<(), String> {
    if chunk.is_empty() {
        return Ok(());
    }

    let simulator = get_global_input_simulator();

    // 获取配置并计算批量大小
    let (delay_ms, jitter_ms, batch_size) = {
        let sim = simulator
            .lock()
            .map_err(|_| "无法获取输入模拟器锁".to_string())?;
        let config = sim.get_config();
        let delay_ms = if config.chars_per_second > 0 {
            1000 / config.chars_per_second as u64
        } else {
            20 // 默认延迟
        };

        // 根据输入速度动态调整批量大小
        let batch_size = if config.chars_per_second > 100 {
            5 // 高速输入时使用较大批量
        } else if config.chars_per_second > 50 {
            3 // 中速输入时使用中等批量
        } else {
            1 // 低速输入时逐字符输入
        };

        (delay_ms, config.jitter_ms, batch_size)
    };

    let chars: Vec<char> = chunk.chars().collect();

    // 分批处理字符
    for batch in chars.chunks(batch_size) {
        // 批量输入字符（减少锁竞争）
        {
            let sim = simulator
                .lock()
                .map_err(|_| "无法获取输入模拟器锁".to_string())?;

            for ch in batch {
                sim.send_unicode_char(*ch)?;
            }
        } // 锁在这里被释放

        // 在批次之间添加延迟（而不是每个字符之间）
        if batch.len() < chars.len() {
            let batch_delay = delay_ms * batch.len() as u64;

            // 添加随机抖动
            let jitter = if jitter_ms > 0 {
                fastrand::u32(0..=jitter_ms) as i32 - (jitter_ms as i32 / 2)
            } else {
                0
            };

            let actual_delay = (batch_delay as i32 + jitter).max(1) as u64;
            tokio::time::sleep(tokio::time::Duration::from_millis(actual_delay)).await;
        }
    }

    Ok(())
}

/// 智能流式输入（根据内容类型优化）
pub async fn simulate_text_chunk_input_smart(chunk: &str) -> Result<(), String> {
    if chunk.is_empty() {
        return Ok(());
    }

    // 分析文本内容特征
    let has_newlines = chunk.contains('\n');
    let has_special_chars = chunk.chars().any(|c| c.is_control() || !c.is_ascii());
    let is_long_text = chunk.len() > 50;

    // 简化的输入策略调试（仅在包含换行符时输出）
    if has_newlines {
        let strategy = if has_newlines || has_special_chars {
            "精确输入"
        } else if is_long_text {
            "批量输入"
        } else {
            "标准输入"
        };
        println!(
            "输入策略: {} (长度={}, 换行符={})",
            strategy,
            chunk.len(),
            has_newlines
        );
    }

    // 根据内容特征选择输入策略
    if has_newlines || has_special_chars {
        // 包含特殊字符的文本使用逐字符输入以确保准确性
        simulate_text_chunk_input_precise(chunk).await
    } else if is_long_text {
        // 长文本使用批量输入以提高效率
        simulate_text_chunk_input_batched(chunk).await
    } else {
        // 短文本使用标准输入
        simulate_text_chunk_input_batched(chunk).await
    }
}

/// 精确流式输入（逐字符，用于特殊内容）
pub async fn simulate_text_chunk_input_precise(chunk: &str) -> Result<(), String> {
    if chunk.is_empty() {
        return Ok(());
    }

    let simulator = get_global_input_simulator();
    let chars: Vec<char> = chunk.chars().collect();

    // 获取配置
    let (delay_ms, jitter_ms) = {
        let sim = simulator
            .lock()
            .map_err(|_| "无法获取输入模拟器锁".to_string())?;
        let config = sim.get_config();
        let delay_ms = if config.chars_per_second > 0 {
            1000 / config.chars_per_second as u64
        } else {
            20
        };
        (delay_ms, config.jitter_ms)
    };

    // 逐字符精确输入
    for (i, ch) in chars.iter().enumerate() {
        {
            let sim = simulator
                .lock()
                .map_err(|_| "无法获取输入模拟器锁".to_string())?;
            sim.send_unicode_char(*ch)?;
        }

        // 除了最后一个字符，都添加延迟
        if i < chars.len() - 1 {
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

/// 验证和安全的文本输入（带验证和恢复机制）
pub async fn simulate_text_input_safe(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }

    // 预处理文本：检查和清理
    let processed_text = preprocess_text_for_input(text)?;

    // 尝试输入文本
    match simulate_text_input(&processed_text).await {
        Ok(()) => {
            // 输入成功，进行后验证（可选）
            validate_text_input_success(&processed_text).await
        }
        Err(e) => {
            // 输入失败，尝试恢复
            handle_text_input_failure(text, &e).await
        }
    }
}

/// 预处理文本以确保输入安全性
fn preprocess_text_for_input(text: &str) -> Result<String, String> {
    // 检查文本长度
    if text.len() > 100_000 {
        return Err("文本过长，超过100KB限制".to_string());
    }

    // 检查是否包含危险字符序列
    if text.contains('\0') {
        return Err("文本包含空字符，可能导致输入错误".to_string());
    }

    // 规范化换行符
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");

    // 移除或替换可能有问题的控制字符
    let cleaned: String = normalized
        .chars()
        .filter(|&c| {
            // 保留可打印字符、空白字符和常见控制字符
            c.is_control() && matches!(c, '\n' | '\t' | '\r') || !c.is_control()
        })
        .collect();

    Ok(cleaned)
}

/// 验证文本输入是否成功
async fn validate_text_input_success(text: &str) -> Result<(), String> {
    // 简单的成功验证：等待一小段时间确保输入完成
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 这里可以添加更复杂的验证逻辑，比如：
    // 1. 检查剪贴板内容是否匹配
    // 2. 使用OCR验证屏幕内容
    // 3. 检查目标应用程序的状态

    println!("文本输入验证通过，长度: {} 字符", text.len());
    Ok(())
}

/// 处理文本输入失败
async fn handle_text_input_failure(original_text: &str, error: &str) -> Result<(), String> {
    println!("文本输入失败: {}", error);

    // 分析失败原因并尝试恢复策略
    if error.contains("锁") {
        // 锁竞争问题：等待后重试
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        return simulate_text_input_safe_retry(original_text, 2).await;
    } else if error.contains("Unicode") || error.contains("字符") {
        // Unicode字符问题：尝试分段输入
        return simulate_text_input_chunked_safe(original_text).await;
    } else if error.contains("SendInput") {
        // Windows API问题：尝试降级输入方式
        return simulate_text_input_fallback(original_text).await;
    }

    // 无法恢复的错误
    Err(format!("文本输入失败且无法恢复: {}", error))
}

/// 安全重试文本输入
async fn simulate_text_input_safe_retry(text: &str, max_retries: u32) -> Result<(), String> {
    for attempt in 1..=max_retries {
        println!("重试文本输入，第 {} 次尝试", attempt);

        match simulate_text_input(text).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                if attempt == max_retries {
                    return Err(format!("重试 {} 次后仍然失败: {}", max_retries, e));
                }

                // 指数退避延迟
                let delay_ms = 200 * (1 << (attempt - 1));
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }
        }
    }

    Err("重试失败".to_string())
}

/// 分段安全输入文本
async fn simulate_text_input_chunked_safe(text: &str) -> Result<(), String> {
    // 将文本分成较小的块
    const CHUNK_SIZE: usize = 100;
    let chars: Vec<char> = text.chars().collect();

    for chunk in chars.chunks(CHUNK_SIZE) {
        let chunk_str: String = chunk.iter().collect();

        // 使用智能输入方法
        match simulate_text_chunk_input_smart(&chunk_str).await {
            Ok(()) => {
                // 在块之间添加小延迟
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
            Err(e) => {
                return Err(format!("分段输入失败: {}", e));
            }
        }
    }

    Ok(())
}

/// 降级输入方式（最后的备选方案）
async fn simulate_text_input_fallback(text: &str) -> Result<(), String> {
    println!("使用降级输入方式");

    // 使用最基本的逐字符输入，忽略所有优化
    let simulator = get_global_input_simulator();
    let chars: Vec<char> = text.chars().collect();

    for ch in chars {
        // 简单的字符输入，不使用批量处理
        {
            let sim = simulator
                .lock()
                .map_err(|_| "无法获取输入模拟器锁".to_string())?;

            // 使用最基本的字符发送方法
            match sim.send_unicode_char(ch) {
                Ok(()) => {}
                Err(e) => {
                    // 如果连基本字符都无法发送，跳过该字符
                    println!("跳过无法输入的字符 '{}': {}", ch, e);
                    continue;
                }
            }
        }

        // 固定延迟
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    Ok(())
}
