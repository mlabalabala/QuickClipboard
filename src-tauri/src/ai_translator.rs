
use crate::ai_config::AIConfig;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct TranslationConfig {
    /// 通用AI配置
    pub ai_config: AIConfig,
    /// 目标语言代码
    pub target_language: String,
    /// 翻译提示词模板
    pub prompt_template: String,
}

impl Default for TranslationConfig {
    fn default() -> Self {
        Self {
            ai_config: AIConfig::default(),
            target_language: "zh-CN".to_string(),
            prompt_template:
                "请将以下文本翻译成{target_language}，严格保持原文的所有格式、换行符、段落结构和空白字符，只返回翻译结果，不要添加任何解释或修改格式："
                    .to_string(),
        }
    }
}

/// 翻译请求
#[derive(Debug, Serialize)]
struct TranslationRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    temperature: f32,
    max_tokens: u32,
}

/// 消息结构
#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

/// 流式响应数据
#[derive(Debug, Deserialize)]
struct StreamResponse {
    id: Option<String>,
    object: Option<String>,
    created: Option<u64>,
    model: Option<String>,
    choices: Option<Vec<Choice>>,
}

/// 选择项
#[derive(Debug, Deserialize)]
struct Choice {
    index: Option<u32>,
    delta: Option<Delta>,
    finish_reason: Option<String>,
}

/// 增量内容
#[derive(Debug, Deserialize)]
struct Delta {
    role: Option<String>,
    content: Option<String>,
}

/// 翻译错误类型
#[derive(Debug)]
pub enum TranslationError {
    NetworkError(reqwest::Error),
    AuthenticationError,
    RateLimitError,
    ParseError(String),
    ConfigError(String),
    TimeoutError,
    UnknownError(String),
}

impl std::fmt::Display for TranslationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranslationError::NetworkError(e) => write!(f, "网络请求失败: {}", e),
            TranslationError::AuthenticationError => write!(f, "API认证失败"),
            TranslationError::RateLimitError => write!(f, "API限流"),
            TranslationError::ParseError(e) => write!(f, "API响应解析失败: {}", e),
            TranslationError::ConfigError(e) => write!(f, "配置错误: {}", e),
            TranslationError::TimeoutError => write!(f, "超时"),
            TranslationError::UnknownError(e) => write!(f, "未知错误: {}", e),
        }
    }
}

impl std::error::Error for TranslationError {}

impl From<reqwest::Error> for TranslationError {
    fn from(error: reqwest::Error) -> Self {
        TranslationError::NetworkError(error)
    }
}

/// 翻译结果
#[derive(Debug)]
pub enum TranslationResult {
    /// 流式内容片段
    Chunk(String),
    /// 翻译完成
    Complete,
    /// 翻译错误
    Error(TranslationError),
}

/// 线程安全的AI翻译器，支持并发使用
pub struct AITranslator {
    client: Client,
    config: TranslationConfig,
}

// 确保 AITranslator 是 Send 和 Sync 的
unsafe impl Send for AITranslator {}
unsafe impl Sync for AITranslator {}

impl AITranslator {
    /// 创建新的AI翻译器
    pub fn new(config: TranslationConfig) -> Result<Self, TranslationError> {
        if !config.ai_config.is_valid() {
            return Err(TranslationError::ConfigError("AI配置无效".to_string()));
        }

        let client = Client::builder()
            .timeout(config.ai_config.timeout())
            .build()
            .map_err(TranslationError::NetworkError)?;

        Ok(Self { client, config })
    }

    /// 翻译文本（非流式，返回完整结果）
    pub async fn translate(&self, text: &str) -> Result<String, TranslationError> {
        let mut receiver = self.translate_stream(text).await?;
        let mut result = String::new();

        // 收集所有流式响应片段
        while let Some(translation_result) = receiver.recv().await {
            match translation_result {
                TranslationResult::Chunk(chunk) => {
                    result.push_str(&chunk);
                }
                TranslationResult::Complete => {
                    break;
                }
                TranslationResult::Error(e) => {
                    return Err(e);
                }
            }
        }

        if result.is_empty() {
            Err(TranslationError::ConfigError("翻译结果为空".to_string()))
        } else {
            Ok(result)
        }
    }

    /// 翻译文本（流式）
    pub async fn translate_stream(
        &self,
        text: &str,
    ) -> Result<mpsc::Receiver<TranslationResult>, TranslationError> {
        let (tx, rx) = mpsc::channel(100);

        let prompt = self
            .config
            .prompt_template
            .replace("{target_language}", &self.config.target_language);

        let request = TranslationRequest {
            model: self.config.ai_config.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: format!("{}\n\n{}", prompt, text),
            }],
            stream: true,
            temperature: self.config.ai_config.temperature,
            max_tokens: self.config.ai_config.max_tokens,
        };

        let url = self.config.ai_config.get_chat_completions_url();
        let client = self.client.clone();
        let api_key = self.config.ai_config.api_key.clone();

        // 调试输出
        println!("AI翻译请求:");
        println!("  URL: {}", url);
        println!("  Model: {}", request.model);
        println!("  Content: {}", request.messages[0].content);
        println!("  API Key: {}...", &api_key[..api_key.len().min(10)]);

        tokio::spawn(async move {
            let result = Self::send_stream_request(client, url, api_key, request, tx.clone()).await;

            if let Err(e) = result {
                let _ = tx.send(TranslationResult::Error(e)).await;
            } else {
                let _ = tx.send(TranslationResult::Complete).await;
            }
        });

        Ok(rx)
    }

    /// 发送流式请求
    async fn send_stream_request(
        client: Client,
        url: String,
        api_key: String,
        request: TranslationRequest,
        tx: mpsc::Sender<TranslationResult>,
    ) -> Result<(), TranslationError> {
        use futures_util::StreamExt;

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();

            // 尝试获取错误响应体以获得更详细的错误信息
            let error_body = match response.text().await {
                Ok(body) => body,
                Err(_) => "无法读取错误响应".to_string(),
            };

            let error = match status.as_u16() {
                400 => {
                    TranslationError::UnknownError(format!("HTTP 400 Bad Request: {}", error_body))
                }
                401 => TranslationError::AuthenticationError,
                429 => TranslationError::RateLimitError,
                _ => TranslationError::UnknownError(format!(
                    "HTTP {} {}: {}",
                    status.as_u16(),
                    status.canonical_reason().unwrap_or("Unknown"),
                    error_body
                )),
            };
            return Err(error);
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk_bytes = chunk_result?;
            let chunk_str = String::from_utf8_lossy(&chunk_bytes);
            buffer.push_str(&chunk_str);

            // 处理SSE格式的数据
            while let Some(line_end) = buffer.find('\n') {
                let line = buffer[..line_end].trim().to_string();
                buffer = buffer[line_end + 1..].to_string();

                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        break;
                    }

                    // 尝试解析JSON响应
                    match serde_json::from_str::<StreamResponse>(data) {
                        Ok(response) => {
                            if let Some(choices) = response.choices {
                                for choice in choices {
                                    if let Some(delta) = choice.delta {
                                        if let Some(content) = delta.content {
                                            if !content.is_empty() {
                                                if let Err(_) =
                                                    tx.send(TranslationResult::Chunk(content)).await
                                                {
                                                    // 接收端已关闭，停止发送
                                                    return Ok(());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            // 解析失败，记录错误但继续处理
                            println!("解析流式响应失败: {} - 数据: {}", e, data);
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// 从应用设置创建翻译配置
pub fn config_from_settings(settings: &crate::settings::AppSettings) -> TranslationConfig {
    let ai_config = crate::ai_config::create_ai_config_from_settings(settings);

    TranslationConfig {
        ai_config,
        target_language: settings.ai_target_language.clone(),
        prompt_template: settings.ai_translation_prompt.clone(),
    }
}

/// 检查翻译配置是否有效
pub fn is_translation_config_valid(settings: &crate::settings::AppSettings) -> bool {
    crate::ai_config::is_ai_config_valid(settings) && !settings.ai_target_language.is_empty()
}
