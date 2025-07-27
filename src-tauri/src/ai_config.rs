//! AI配置管理模块
//!
//! 本模块提供通用的AI配置管理功能，可以被各种AI功能复用。
//! 包括API配置、模型管理、请求参数等。
//!
//! ## 主要功能
//! - **AI配置结构定义**: 定义通用的AI配置结构
//! - **配置验证**: 验证AI配置的有效性
//! - **模型管理**: 获取可用模型列表
//! - **配置转换**: 从应用设置转换为AI配置
//!
//! ## 使用示例
//! ```rust
//! use crate::ai_config::{AIConfig, create_ai_config_from_settings};
//!
//! let settings = get_global_settings();
//! let ai_config = create_ai_config_from_settings(&settings);
//!
//! if ai_config.is_valid() {
//!     // 使用AI配置
//! }
//! ```

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// AI配置结构体
///
/// 包含AI服务所需的所有配置参数，包括API认证信息、模型选择、
/// 请求参数等。这是一个通用的配置结构，可以被各种AI功能复用。
///
/// ## 字段说明
/// - `api_key`: API密钥，用于身份认证
/// - `model`: 要使用的AI模型名称
/// - `base_url`: API服务的基础URL
/// - `timeout`: API请求超时时间
/// - `temperature`: 模型温度参数，控制输出的随机性
/// - `max_tokens`: 最大输出token数量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    /// API密钥，用于身份认证
    pub api_key: String,
    /// AI模型名称
    pub model: String,
    /// API服务基础URL
    pub base_url: String,
    /// 请求超时时间（秒）
    pub timeout_secs: u64,
    /// 模型温度参数 (0.0-2.0)
    pub temperature: f32,
    /// 最大输出token数量
    pub max_tokens: u32,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "Qwen/Qwen2-7B-Instruct".to_string(),
            base_url: "https://api.siliconflow.cn/v1".to_string(),
            timeout_secs: 120,
            temperature: 0.3,
            max_tokens: 2048,
        }
    }
}

impl AIConfig {
    /// 创建新的AI配置
    pub fn new(api_key: String, model: String, base_url: String) -> Self {
        Self {
            api_key,
            model,
            base_url,
            ..Default::default()
        }
    }

    /// 验证配置是否有效
    pub fn is_valid(&self) -> bool {
        !self.api_key.trim().is_empty()
            && !self.model.trim().is_empty()
            && !self.base_url.trim().is_empty()
            && self.timeout_secs > 0
            && self.temperature >= 0.0
            && self.temperature <= 2.0
            && self.max_tokens > 0
    }

    /// 获取超时时间
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }

    /// 设置温度参数
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// 设置最大token数量
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// 设置超时时间
    pub fn with_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// 获取API完整URL
    pub fn get_chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    /// 获取模型列表URL
    pub fn get_models_url(&self) -> String {
        format!("{}/models", self.base_url.trim_end_matches('/'))
    }
}

/// AI模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIModel {
    pub id: String,
    pub object: String,
    pub created: Option<u64>,
    pub owned_by: Option<String>,
}

/// 模型列表响应
#[derive(Debug, Deserialize)]
pub struct ModelsResponse {
    pub data: Vec<AIModel>,
    pub object: String,
}

/// AI配置管理器
pub struct AIConfigManager {
    config: AIConfig,
    client: Client,
}

impl AIConfigManager {
    /// 创建新的AI配置管理器
    pub fn new(config: AIConfig) -> Result<Self, String> {
        if !config.is_valid() {
            return Err("AI配置无效".to_string());
        }

        let client = Client::builder()
            .timeout(config.timeout())
            .build()
            .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;

        Ok(Self { config, client })
    }

    /// 获取当前配置
    pub fn get_config(&self) -> &AIConfig {
        &self.config
    }

    /// 更新配置
    pub fn update_config(&mut self, config: AIConfig) -> Result<(), String> {
        if !config.is_valid() {
            return Err("AI配置无效".to_string());
        }

        // 如果超时时间发生变化，需要重新创建客户端
        if config.timeout_secs != self.config.timeout_secs {
            self.client = Client::builder()
                .timeout(config.timeout())
                .build()
                .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;
        }

        self.config = config;
        Ok(())
    }

    /// 获取可用模型列表
    pub async fn get_available_models(&self) -> Result<Vec<String>, String> {
        let url = self.config.get_models_url();

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| format!("API请求失败: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API请求失败，状态码: {}", response.status()));
        }

        let models_response: ModelsResponse = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败: {}", e))?;

        let model_ids: Vec<String> = models_response
            .data
            .into_iter()
            .map(|model| model.id)
            .collect();

        Ok(model_ids)
    }

    /// 测试配置是否可用
    pub async fn test_config(&self) -> Result<(), String> {
        // 尝试获取模型列表来测试配置
        self.get_available_models().await?;
        Ok(())
    }
}

/// 从应用设置创建AI配置
pub fn create_ai_config_from_settings(settings: &crate::settings::AppSettings) -> AIConfig {
    AIConfig {
        api_key: settings.ai_api_key.clone(),
        model: settings.ai_model.clone(),
        base_url: settings.ai_base_url.clone(),
        timeout_secs: 120,
        temperature: 0.3,
        max_tokens: 2048,
    }
}

/// 检查AI配置是否有效
pub fn is_ai_config_valid(settings: &crate::settings::AppSettings) -> bool {
    let config = create_ai_config_from_settings(settings);
    config.is_valid()
}

/// 获取推荐的AI模型列表
pub fn get_recommended_models() -> Vec<&'static str> {
    vec![
        "Qwen/Qwen2-7B-Instruct",
        "deepseek-v3",
        "deepseek-chat",
        "deepseek-coder",
        "qwen-turbo",
        "qwen-plus",
        "qwen-max",
        "qwen2.5-72b-instruct",
        "qwen2.5-32b-instruct",
        "qwen2.5-14b-instruct",
        "qwen2.5-7b-instruct",
    ]
}

/// 获取模型的友好显示名称
pub fn get_model_display_name(model_id: &str) -> String {
    match model_id {
        "Qwen/Qwen2-7B-Instruct" => "Qwen2-7B-Instruct（推荐）".to_string(),
        "deepseek-v3" => "DeepSeek V3".to_string(),
        "deepseek-chat" => "DeepSeek Chat".to_string(),
        "deepseek-coder" => "DeepSeek Coder".to_string(),
        "qwen-turbo" => "通义千问 Turbo".to_string(),
        "qwen-plus" => "通义千问 Plus".to_string(),
        "qwen-max" => "通义千问 Max".to_string(),
        "qwen2.5-72b-instruct" => "Qwen2.5-72B-Instruct".to_string(),
        "qwen2.5-32b-instruct" => "Qwen2.5-32B-Instruct".to_string(),
        "qwen2.5-14b-instruct" => "Qwen2.5-14B-Instruct".to_string(),
        "qwen2.5-7b-instruct" => "Qwen2.5-7B-Instruct".to_string(),
        "chatglm3-6b" => "ChatGLM3-6B".to_string(),
        "yi-34b-chat" => "Yi-34B-Chat".to_string(),
        "yi-6b-chat" => "Yi-6B-Chat".to_string(),
        "baichuan2-13b-chat" => "Baichuan2-13B-Chat".to_string(),
        "internlm2-chat-7b" => "InternLM2-Chat-7B".to_string(),
        "internlm2-chat-20b" => "InternLM2-Chat-20B".to_string(),
        _ => model_id.to_string(),
    }
}
