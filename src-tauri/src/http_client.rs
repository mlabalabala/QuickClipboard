use reqwest;
use serde_json::Value;
use std::time::Duration;

/// HTTP客户端配置
pub struct HttpClientConfig {
    pub base_url: String,
    pub timeout_seconds: u64,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:18080".to_string(),
            timeout_seconds: 10,
        }
    }
}

/// HTTP客户端
pub struct HttpClient {
    client: reqwest::Client,
    config: HttpClientConfig,
}

impl HttpClient {
    /// 创建新的HTTP客户端
    pub fn new(config: HttpClientConfig) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;

        Ok(Self { client, config })
    }

    /// 创建默认HTTP客户端
    pub fn default() -> Result<Self, String> {
        Self::new(HttpClientConfig::default())
    }

    /// 发送GET请求
    pub async fn get(&self, endpoint: &str) -> Result<Value, String> {
        let url = format!("{}{}", self.config.base_url, endpoint);
        
        // println!("发送GET请求: {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("GET请求失败: {}", e))?;

        if response.status().is_success() {
            let json = response
                .json::<Value>()
                .await
                .map_err(|e| format!("解析响应JSON失败: {}", e))?;
            Ok(json)
        } else {
            Err(format!("HTTP请求失败，状态码: {}", response.status()))
        }
    }

    /// 发送POST请求
    pub async fn post(&self, endpoint: &str, body: Option<Value>) -> Result<Value, String> {
        let url = format!("{}{}", self.config.base_url, endpoint);
        
        // println!("发送POST请求: {}", url);
        
        let mut request = self.client.post(&url);
        
        if let Some(json_body) = body {
            request = request.json(&json_body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("POST请求失败: {}", e))?;

        if response.status().is_success() {
            // 尝试解析JSON，如果失败则返回文本响应
            let text = response
                .text()
                .await
                .map_err(|e| format!("获取响应文本失败: {}", e))?;
            
            // 尝试解析为JSON
            match serde_json::from_str::<Value>(&text) {
                Ok(json) => Ok(json),
                Err(_) => {
                    // 如果不是JSON，返回包含文本的JSON对象
                    Ok(serde_json::json!({ "message": text, "success": true }))
                }
            }
        } else {
            Err(format!("HTTP请求失败，状态码: {}", response.status()))
        }
    }

    /// 发送PUT请求
    pub async fn put(&self, endpoint: &str, body: Option<Value>) -> Result<Value, String> {
        let url = format!("{}{}", self.config.base_url, endpoint);
        
        println!("发送PUT请求: {}", url);
        
        let mut request = self.client.put(&url);
        
        if let Some(json_body) = body {
            request = request.json(&json_body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("PUT请求失败: {}", e))?;

        if response.status().is_success() {
            let json = response
                .json::<Value>()
                .await
                .map_err(|e| format!("解析响应JSON失败: {}", e))?;
            Ok(json)
        } else {
            Err(format!("HTTP请求失败，状态码: {}", response.status()))
        }
    }

    /// 发送DELETE请求
    pub async fn delete(&self, endpoint: &str) -> Result<Value, String> {
        let url = format!("{}{}", self.config.base_url, endpoint);
        
        println!("发送DELETE请求: {}", url);
        
        let response = self.client
            .delete(&url)
            .send()
            .await
            .map_err(|e| format!("DELETE请求失败: {}", e))?;

        if response.status().is_success() {
            let json = response
                .json::<Value>()
                .await
                .map_err(|e| format!("解析响应JSON失败: {}", e))?;
            Ok(json)
        } else {
            Err(format!("HTTP请求失败，状态码: {}", response.status()))
        }
    }

    /// 发送简单的POST请求（不解析响应JSON，只检查状态码）
    pub async fn post_simple(&self, endpoint: &str) -> Result<(), String> {
        let url = format!("{}{}", self.config.base_url, endpoint);
        
        let response = self.client
            .post(&url)
            .send()
            .await
            .map_err(|e| format!("POST请求失败: {}", e))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("HTTP请求失败，状态码: {}", response.status()))
        }
    }

    /// 检查服务是否可用
    pub async fn health_check(&self) -> bool {
        match self.get("/health").await {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}
