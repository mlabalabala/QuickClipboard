use crate::http_client::{HttpClient, HttpClientConfig};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// 外部API端点定义
/// 所有与子程序交互的API都在这里集中定义，方便开发时查看和维护
pub struct ExternalApis;

impl ExternalApis {
    /// 截屏API端点
    pub const SCREENSHOT: &'static str = "/screenshot";
    
    // 在这里添加新的API端点，保持代码整洁和可维护性
}

/// 外部API服务
/// 负责与子程序进行HTTP通信的所有逻辑
pub struct ExternalApiService {
    client: HttpClient,
}

impl ExternalApiService {
    /// 创建新的外部API服务
    pub fn new() -> Result<Self, String> {
        let port = get_external_service_port()
            .ok_or_else(|| "外部服务端口未设置，请先启动子程序".to_string())?;
            
        let config = HttpClientConfig {
            base_url: format!("http://localhost:{}", port),
            timeout_seconds: 3,
        };
        
        let client = HttpClient::new(config)?;
        
        Ok(Self { client })
    }

    /// 触发截屏
    pub async fn trigger_screenshot(&self) -> Result<(), String> {
        self.client.post(ExternalApis::SCREENSHOT, None).await?;
        Ok(())
    }
}

// =================== 全局状态管理 ===================

// 全局状态存储动态端口
static EXTERNAL_SERVICE_PORT: Lazy<Arc<Mutex<Option<u16>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(None))
});

/// 设置外部服务端口
pub fn set_external_service_port(port: u16) {
    if let Ok(mut port_guard) = EXTERNAL_SERVICE_PORT.lock() {
        *port_guard = Some(port);
        println!("外部服务端口已设置为: {}", port);
    }
}

/// 获取外部服务端口
pub fn get_external_service_port() -> Option<u16> {
    EXTERNAL_SERVICE_PORT.lock().ok().and_then(|guard| *guard)
}

/// 快速触发截屏（直接调用）
pub async fn trigger_screenshot() -> Result<(), String> {
    let service = ExternalApiService::new()?;
    service.trigger_screenshot().await?;
    
    Ok(())
}