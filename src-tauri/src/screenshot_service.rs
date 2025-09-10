use crate::http_client::{HttpClient, HttpClientConfig};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// 截屏API端点定义
/// 所有与截屏程序交互的API都在这里集中定义
pub struct ScreenshotApis;

impl ScreenshotApis {
    /// 截屏API端点
    pub const SCREENSHOT: &'static str = "/screenshot";
    
    // 在这里添加新的截屏相关API端点，保持代码整洁和可维护性
}

/// 截屏API服务
/// 负责与截屏程序进行HTTP通信的所有逻辑
pub struct ScreenshotApiService {
    client: HttpClient,
}

impl ScreenshotApiService {
    /// 创建新的截屏API服务
    pub fn new() -> Result<Self, String> {
        let port = get_screenshot_service_port()
            .ok_or_else(|| "截屏服务端口未设置，请先启动截屏程序".to_string())?;
            
        let config = HttpClientConfig {
            base_url: format!("http://localhost:{}", port),
            timeout_seconds: 3,
        };
        
        let client = HttpClient::new(config)?;
        
        Ok(Self { client })
    }

    /// 触发截屏
    pub async fn trigger_screenshot(&self) -> Result<(), String> {
        self.client.post(ScreenshotApis::SCREENSHOT, None).await?;
        Ok(())
    }
}

// =================== 截屏服务状态管理 ===================

// 全局状态存储截屏服务动态端口
static SCREENSHOT_SERVICE_PORT: Lazy<Arc<Mutex<Option<u16>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(None))
});

/// 设置截屏服务端口
pub fn set_screenshot_service_port(port: u16) {
    if let Ok(mut port_guard) = SCREENSHOT_SERVICE_PORT.lock() {
        *port_guard = Some(port);
        println!("截屏服务端口已设置为: {}", port);
    }
}

/// 获取截屏服务端口
pub fn get_screenshot_service_port() -> Option<u16> {
    SCREENSHOT_SERVICE_PORT.lock().ok().and_then(|guard| *guard)
}

/// 快速触发截屏（直接调用）
pub async fn trigger_screenshot() -> Result<(), String> {
    let service = ScreenshotApiService::new()?;
    service.trigger_screenshot().await?;
    
    Ok(())
}