use crate::http_client::{HttpClient, HttpClientConfig};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

/// 截屏API端点定义
/// 所有与截屏程序交互的API都在这里集中定义
pub struct ScreenshotApis;

impl ScreenshotApis {
    /// 截屏API端点
    pub const SCREENSHOT: &'static str = "/screenshot";
    /// 心跳检测API端点
    pub const HEARTBEAT: &'static str = "/heartbeat";
    /// 钉图片API端点
    pub const PIN_IMAGE: &'static str = "/pinimage";
    
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

    /// 发送心跳检测请求
    pub async fn send_heartbeat(&self) -> Result<(), String> {
        self.client.get(ScreenshotApis::HEARTBEAT).await?;
        Ok(())
    }

    /// 钉图片到屏幕
    pub async fn pin_image(&self, image_path: &str) -> Result<(), String> {
        let url = format!("{}?path={}", ScreenshotApis::PIN_IMAGE, urlencoding::encode(image_path));
        self.client.post_simple(&url).await?;
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

/// 设置截屏服务端口并启动心跳检测
pub fn set_screenshot_service_port_and_start_heartbeat(port: u16, app_handle: tauri::AppHandle) {
    // 先设置端口
    set_screenshot_service_port(port);
    
    // 等待一小段时间确保端口设置完成，然后启动心跳检测
    let app_handle_clone = app_handle.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(500));
        start_heartbeat_service(app_handle_clone);
    });
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

/// 快速钉图片（直接调用）
pub async fn pin_image_to_screen(image_path: &str) -> Result<(), String> {
    let service = ScreenshotApiService::new()?;
    service.pin_image(image_path).await?;
    
    Ok(())
}

// =================== 心跳检测管理 ===================

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use tokio::time::{sleep, Duration};

/// 心跳检测状态
static HEARTBEAT_RUNNING: AtomicBool = AtomicBool::new(false);
/// 心跳失败计数器
static HEARTBEAT_FAILURE_COUNT: AtomicU32 = AtomicU32::new(0);
/// 最大失败次数，超过此数值将重启截屏程序
const MAX_HEARTBEAT_FAILURES: u32 = 3;
/// 心跳间隔（秒）
const HEARTBEAT_INTERVAL_SECONDS: u64 = 2;

/// 启动心跳检测服务
pub fn start_heartbeat_service(app_handle: tauri::AppHandle) {
    // 防止重复启动
    if HEARTBEAT_RUNNING.compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed).is_err() {
        println!("心跳检测服务已在运行中");
        return;
    }

    println!("启动心跳检测服务，间隔: {}秒", HEARTBEAT_INTERVAL_SECONDS);
    
    // 在新线程中创建tokio runtime来运行心跳检测
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("创建tokio runtime失败");
        rt.block_on(async move {
            while HEARTBEAT_RUNNING.load(Ordering::Relaxed) {
                // 等待心跳间隔
                sleep(Duration::from_secs(HEARTBEAT_INTERVAL_SECONDS)).await;
                
                // 检查是否还在运行
                if !HEARTBEAT_RUNNING.load(Ordering::Relaxed) {
                    break;
                }
                
                // 尝试发送心跳
                match send_heartbeat_internal().await {
                    Ok(()) => {
                        // 心跳成功，重置失败计数器
                        HEARTBEAT_FAILURE_COUNT.store(0, Ordering::Relaxed);
                        println!("心跳检测成功");
                    }
                    Err(e) => {
                        // 心跳失败，增加失败计数
                        let failure_count = HEARTBEAT_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
                        println!("心跳检测失败 ({}次): {}", failure_count, e);
                        
                        // 检查是否超过最大失败次数
                        if failure_count >= MAX_HEARTBEAT_FAILURES {
                            println!("心跳检测失败次数过多，尝试重启截屏程序");
                            
                            // 重置失败计数器
                            HEARTBEAT_FAILURE_COUNT.store(0, Ordering::Relaxed);
                            
                            // 尝试重启截屏程序
                            if let Err(restart_error) = crate::commands::launch_external_screenshot_process(app_handle.clone()) {
                                println!("重启截屏程序失败: {}", restart_error);
                            } else {
                                println!("截屏程序重启成功");
                            }
                        }
                    }
                }
            }
            
            println!("心跳检测服务已停止");
        });
    });
}

/// 内部心跳检测实现
async fn send_heartbeat_internal() -> Result<(), String> {
    let service = ScreenshotApiService::new()?;
    service.send_heartbeat().await?;
    Ok(())
}