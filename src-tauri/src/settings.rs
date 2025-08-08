use dirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    // 基础设置
    pub auto_start: bool,
    pub start_hidden: bool,
    pub run_as_admin: bool,
    pub show_startup_notification: bool,
    pub history_limit: u64,
    pub theme: String,
    pub opacity: f64,
    pub toggle_shortcut: String,
    pub number_shortcuts: bool,
    pub clipboard_monitor: bool,
    pub ignore_duplicates: bool,
    pub save_images: bool,
    pub show_image_preview: bool,

    // 音效设置
    pub sound_enabled: bool,
    pub sound_volume: f64,
    pub copy_sound_path: String,
    pub paste_sound_path: String,
    pub sound_preset: String,

    // 截屏设置
    pub screenshot_enabled: bool,
    pub screenshot_shortcut: String,
    pub screenshot_quality: u8,
    pub screenshot_auto_save: bool,
    pub screenshot_show_hints: bool,

    // 预览窗口设置
    pub preview_enabled: bool,
    pub preview_shortcut: String,
    pub preview_items_count: u32,
    pub preview_auto_paste: bool,
    pub preview_scroll_sound: bool,
    pub preview_scroll_sound_path: String,

    // AI翻译设置
    pub ai_translation_enabled: bool,
    pub ai_api_key: String,
    pub ai_model: String,
    pub ai_base_url: String,
    pub ai_target_language: String,
    pub ai_translate_on_copy: bool,
    pub ai_translate_on_paste: bool,
    pub ai_translation_prompt: String,
    pub ai_input_speed: u32,     // 输入速度，字符/秒
    pub ai_newline_mode: String, // 换行符处理模式：auto, enter, shift_enter, unicode
    pub ai_output_mode: String,  // 输出模式：stream（流式输出）, paste（直接粘贴）

    // 鼠标设置
    pub mouse_middle_button_enabled: bool, // 启用鼠标中键显示剪贴板
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            // 基础设置默认值
            auto_start: false,
            start_hidden: true,
            run_as_admin: false,
            show_startup_notification: true,
            history_limit: 100,
            theme: "light".to_string(),
            opacity: 0.9,
            toggle_shortcut: "Win+V".to_string(),
            number_shortcuts: true,
            clipboard_monitor: true,
            ignore_duplicates: true,
            save_images: true,
            show_image_preview: false,

            // 音效设置默认值
            sound_enabled: true,
            sound_volume: 50.0,
            copy_sound_path: "".to_string(),
            paste_sound_path: "".to_string(),
            sound_preset: "default".to_string(),

            // 截屏设置默认值
            screenshot_enabled: true,
            screenshot_shortcut: "Ctrl+Shift+A".to_string(),
            screenshot_quality: 85,
            screenshot_auto_save: true,
            screenshot_show_hints: true,

            // 预览窗口设置默认值
            preview_enabled: true,
            preview_shortcut: "Ctrl+`".to_string(),
            preview_items_count: 5,
            preview_auto_paste: true,
            preview_scroll_sound: true,
            preview_scroll_sound_path: "sounds/roll.mp3".to_string(),

            // AI翻译设置默认值
            ai_translation_enabled: false,
            ai_api_key: "".to_string(),
            ai_model: "Qwen/Qwen2-7B-Instruct".to_string(),
            ai_base_url: "https://api.siliconflow.cn/v1".to_string(),
            ai_target_language: "auto".to_string(),
            ai_translate_on_copy: false,
            ai_translate_on_paste: true,
            ai_translation_prompt:
                "请将以下文本翻译成{target_language}，严格保持原文的所有格式、换行符、段落结构和空白字符，只返回翻译结果，不要添加任何解释或修改格式："
                    .to_string(),
            ai_input_speed: 50,                // 50字符/秒
            ai_newline_mode: "auto".to_string(), // 默认使用自动模式，兼容性最好
            ai_output_mode: "stream".to_string(), // 默认使用流式输出

            // 鼠标设置默认值
            mouse_middle_button_enabled: true, // 默认启用鼠标中键显示剪贴板
        }
    }
}

impl AppSettings {
    /// 获取设置文件路径
    fn get_settings_file_path() -> Result<PathBuf, String> {
        // 使用本地数据目录 (AppData\Local\quickclipboard)，与其他组件保持一致
        let config_dir = dirs::data_local_dir()
            .ok_or("无法获取本地数据目录")?
            .join("quickclipboard");

        // 确保配置目录存在
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir).map_err(|e| format!("创建配置目录失败: {}", e))?;
        }

        Ok(config_dir.join("settings.json"))
    }

    /// 从文件加载设置
    pub fn load() -> Self {
        match Self::load_from_file() {
            Ok(settings) => settings,
            Err(_e) => {
                let default_settings = Self::default();
                // 自动保存默认设置到文件
                let _ = default_settings.save();
                default_settings
            }
        }
    }

    /// 从文件加载设置（内部方法）
    fn load_from_file() -> Result<Self, String> {
        let settings_path = Self::get_settings_file_path()?;

        if !settings_path.exists() {
            return Err("设置文件不存在".to_string());
        }

        let content =
            fs::read_to_string(&settings_path).map_err(|e| format!("读取设置文件失败: {}", e))?;

        let settings: AppSettings =
            serde_json::from_str(&content).map_err(|e| format!("解析设置文件失败: {}", e))?;

        Ok(settings)
    }

    /// 保存设置到文件
    pub fn save(&self) -> Result<(), String> {
        let settings_path = Self::get_settings_file_path()?;

        let content =
            serde_json::to_string_pretty(self).map_err(|e| format!("序列化设置失败: {}", e))?;

        fs::write(&settings_path, content).map_err(|e| format!("写入设置文件失败: {}", e))?;

        // println!("设置已保存到: {:?}", settings_path);
        Ok(())
    }

    /// 转换为JSON值（用于前端）
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "autoStart": self.auto_start,
            "startHidden": self.start_hidden,
            "runAsAdmin": self.run_as_admin,
            "showStartupNotification": self.show_startup_notification,
            "historyLimit": self.history_limit,
            "theme": self.theme,
            "opacity": self.opacity,
            "toggleShortcut": self.toggle_shortcut,
            "numberShortcuts": self.number_shortcuts,
            "clipboardMonitor": self.clipboard_monitor,
            "ignoreDuplicates": self.ignore_duplicates,
            "saveImages": self.save_images,
            "showImagePreview": self.show_image_preview,
            "soundEnabled": self.sound_enabled,
            "soundVolume": self.sound_volume,
            "copySoundPath": self.copy_sound_path,
            "pasteSoundPath": self.paste_sound_path,
            "soundPreset": self.sound_preset,
            "screenshot_enabled": self.screenshot_enabled,
            "screenshot_shortcut": self.screenshot_shortcut,
            "screenshot_quality": self.screenshot_quality,
            "screenshot_auto_save": self.screenshot_auto_save,
            "screenshot_show_hints": self.screenshot_show_hints,
            "previewEnabled": self.preview_enabled,
            "previewShortcut": self.preview_shortcut,
            "previewItemsCount": self.preview_items_count,
            "previewAutoPaste": self.preview_auto_paste,
            "previewScrollSound": self.preview_scroll_sound,
            "previewScrollSoundPath": self.preview_scroll_sound_path,
            "aiTranslationEnabled": self.ai_translation_enabled,
            "aiApiKey": self.ai_api_key,
            "aiModel": self.ai_model,
            "aiBaseUrl": self.ai_base_url,
            "aiTargetLanguage": self.ai_target_language,
            "aiTranslateOnCopy": self.ai_translate_on_copy,
            "aiTranslateOnPaste": self.ai_translate_on_paste,
            "aiTranslationPrompt": self.ai_translation_prompt,
            "aiInputSpeed": self.ai_input_speed,
            "aiNewlineMode": self.ai_newline_mode,
            "aiOutputMode": self.ai_output_mode,
            "mouseMiddleButtonEnabled": self.mouse_middle_button_enabled,
        })
    }

    /// 从JSON值更新设置（来自前端）
    pub fn update_from_json(&mut self, json: &serde_json::Value) {
        if let Some(v) = json.get("autoStart").and_then(|v| v.as_bool()) {
            self.auto_start = v;
        }
        if let Some(v) = json.get("startHidden").and_then(|v| v.as_bool()) {
            self.start_hidden = v;
        }
        if let Some(v) = json.get("runAsAdmin").and_then(|v| v.as_bool()) {
            self.run_as_admin = v;
        }
        if let Some(v) = json
            .get("showStartupNotification")
            .and_then(|v| v.as_bool())
        {
            self.show_startup_notification = v;
        }
        if let Some(v) = json.get("historyLimit").and_then(|v| v.as_u64()) {
            self.history_limit = v;
        }
        if let Some(v) = json.get("theme").and_then(|v| v.as_str()) {
            self.theme = v.to_string();
        }
        if let Some(v) = json.get("opacity").and_then(|v| v.as_f64()) {
            self.opacity = v;
        }
        if let Some(v) = json.get("toggleShortcut").and_then(|v| v.as_str()) {
            self.toggle_shortcut = v.to_string();
        }
        if let Some(v) = json.get("numberShortcuts").and_then(|v| v.as_bool()) {
            self.number_shortcuts = v;
        }
        if let Some(v) = json.get("clipboardMonitor").and_then(|v| v.as_bool()) {
            self.clipboard_monitor = v;
        }
        if let Some(v) = json.get("ignoreDuplicates").and_then(|v| v.as_bool()) {
            self.ignore_duplicates = v;
        }
        if let Some(v) = json.get("saveImages").and_then(|v| v.as_bool()) {
            self.save_images = v;
        }
        if let Some(v) = json.get("showImagePreview").and_then(|v| v.as_bool()) {
            self.show_image_preview = v;
        }

        // 音效设置
        if let Some(v) = json.get("soundEnabled").and_then(|v| v.as_bool()) {
            self.sound_enabled = v;
        }
        if let Some(v) = json.get("soundVolume").and_then(|v| v.as_f64()) {
            self.sound_volume = v;
        }
        if let Some(v) = json.get("copySoundPath").and_then(|v| v.as_str()) {
            self.copy_sound_path = v.to_string();
        }
        if let Some(v) = json.get("pasteSoundPath").and_then(|v| v.as_str()) {
            self.paste_sound_path = v.to_string();
        }
        if let Some(v) = json.get("soundPreset").and_then(|v| v.as_str()) {
            self.sound_preset = v.to_string();
        }

        // 截屏设置
        if let Some(v) = json.get("screenshot_enabled").and_then(|v| v.as_bool()) {
            self.screenshot_enabled = v;
        }
        if let Some(v) = json.get("screenshot_shortcut").and_then(|v| v.as_str()) {
            self.screenshot_shortcut = v.to_string();
        }
        if let Some(v) = json.get("screenshot_quality").and_then(|v| v.as_u64()) {
            self.screenshot_quality = v as u8;
        }
        if let Some(v) = json.get("screenshot_auto_save").and_then(|v| v.as_bool()) {
            self.screenshot_auto_save = v;
        }
        if let Some(v) = json.get("screenshot_show_hints").and_then(|v| v.as_bool()) {
            self.screenshot_show_hints = v;
        }

        // 预览窗口设置
        if let Some(v) = json.get("previewEnabled").and_then(|v| v.as_bool()) {
            self.preview_enabled = v;
        }
        if let Some(v) = json.get("previewShortcut").and_then(|v| v.as_str()) {
            self.preview_shortcut = v.to_string();
        }
        if let Some(v) = json.get("previewItemsCount").and_then(|v| v.as_u64()) {
            self.preview_items_count = v as u32;
        }
        if let Some(v) = json.get("previewAutoPaste").and_then(|v| v.as_bool()) {
            self.preview_auto_paste = v;
        }
        if let Some(v) = json.get("previewScrollSound").and_then(|v| v.as_bool()) {
            self.preview_scroll_sound = v;
        }
        if let Some(v) = json.get("previewScrollSoundPath").and_then(|v| v.as_str()) {
            self.preview_scroll_sound_path = v.to_string();
        }

        // AI翻译设置
        if let Some(v) = json.get("aiTranslationEnabled").and_then(|v| v.as_bool()) {
            self.ai_translation_enabled = v;
        }
        if let Some(v) = json.get("aiApiKey").and_then(|v| v.as_str()) {
            self.ai_api_key = v.to_string();
        }
        if let Some(v) = json.get("aiModel").and_then(|v| v.as_str()) {
            self.ai_model = v.to_string();
        }
        if let Some(v) = json.get("aiBaseUrl").and_then(|v| v.as_str()) {
            self.ai_base_url = v.to_string();
        }
        if let Some(v) = json.get("aiTargetLanguage").and_then(|v| v.as_str()) {
            self.ai_target_language = v.to_string();
        }
        if let Some(v) = json.get("aiTranslateOnCopy").and_then(|v| v.as_bool()) {
            self.ai_translate_on_copy = v;
        }
        if let Some(v) = json.get("aiTranslateOnPaste").and_then(|v| v.as_bool()) {
            self.ai_translate_on_paste = v;
        }
        if let Some(v) = json.get("aiTranslationPrompt").and_then(|v| v.as_str()) {
            self.ai_translation_prompt = v.to_string();
        }
        if let Some(v) = json.get("aiInputSpeed").and_then(|v| v.as_u64()) {
            self.ai_input_speed = v as u32;
        }
        if let Some(v) = json.get("aiNewlineMode").and_then(|v| v.as_str()) {
            self.ai_newline_mode = v.to_string();
        }
        if let Some(v) = json.get("aiOutputMode").and_then(|v| v.as_str()) {
            self.ai_output_mode = v.to_string();
        }
        if let Some(v) = json
            .get("mouseMiddleButtonEnabled")
            .and_then(|v| v.as_bool())
        {
            self.mouse_middle_button_enabled = v;
        }
    }
}

// 全局设置实例
use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};

static GLOBAL_SETTINGS: Lazy<Arc<Mutex<AppSettings>>> =
    Lazy::new(|| Arc::new(Mutex::new(AppSettings::load())));

/// 获取全局设置
pub fn get_global_settings() -> AppSettings {
    GLOBAL_SETTINGS.lock().unwrap().clone()
}

/// 更新全局设置
pub fn update_global_settings(settings: AppSettings) -> Result<(), String> {
    {
        let mut global_settings = GLOBAL_SETTINGS.lock().unwrap();
        *global_settings = settings.clone();
    }

    // 保存到文件
    settings.save()?;

    Ok(())
}

/// 从JSON更新全局设置
pub fn update_global_settings_from_json(json: &serde_json::Value) -> Result<(), String> {
    let mut settings = get_global_settings();
    settings.update_from_json(json);
    update_global_settings(settings)
}
