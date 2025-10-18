
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    // API密钥，用于身份认证
    pub api_key: String,
    // AI模型名称
    pub model: String,
    // API服务基础URL
    pub base_url: String,
    // 请求超时时间（秒）
    pub timeout_secs: u64,
    // 模型温度参数 (0.0-2.0)
    pub temperature: f32,
    // 最大输出token数量
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
    // 创建新的AI配置
    pub fn new(api_key: String, model: String, base_url: String) -> Self {
        Self {
            api_key,
            model,
            base_url,
            ..Default::default()
        }
    }

    // 验证配置是否有效
    pub fn is_valid(&self) -> bool {
        !self.api_key.trim().is_empty()
            && !self.model.trim().is_empty()
            && !self.base_url.trim().is_empty()
            && self.timeout_secs > 0
            && self.temperature >= 0.0
            && self.temperature <= 2.0
            && self.max_tokens > 0
    }

    // 获取超时时间
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }

    // 获取API完整URL
    pub fn get_chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    // 获取模型列表URL
    pub fn get_models_url(&self) -> String {
        format!("{}/models", self.base_url.trim_end_matches('/'))
    }
}

// AI模型信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIModel {
    pub id: String,
    pub object: String,
    pub created: Option<u64>,
    pub owned_by: Option<String>,
}

// 模型列表响应
#[derive(Debug, Deserialize)]
pub struct ModelsResponse {
    pub data: Vec<AIModel>,
    pub object: String,
}

// AI配置管理器
pub struct AIConfigManager {
    config: AIConfig,
    client: Client,
}

impl AIConfigManager {
    // 创建新的AI配置管理器
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

    // 获取可用模型列表
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

    // 测试配置是否可用
    pub async fn test_config(&self) -> Result<(), String> {
        // 尝试获取模型列表来测试配置
        self.get_available_models().await?;
        Ok(())
    }
}

// 从应用设置创建AI配置
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

// 检查AI配置是否有效
pub fn is_ai_config_valid(settings: &crate::settings::AppSettings) -> bool {
    let config = create_ai_config_from_settings(settings);
    config.is_valid()
}
