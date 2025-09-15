/// 截图服务 - 处理截图相关的业务逻辑
pub struct ScreenshotService;

impl ScreenshotService {
    /// 启动外部截图进程
    pub fn launch_external_screenshot_process(app: tauri::AppHandle) -> Result<(), String> {
        use std::process::{Command, Stdio};
        use std::io::{BufRead, BufReader};
        use tauri::Manager;
        
        // 获取应用资源目录
        let resource_dir = app.path().resource_dir()
            .map_err(|e| format!("获取资源目录失败: {}", e))?;
        
        // 构建external_apps文件夹路径
        let external_apps_dir = resource_dir.join("external_apps");
        
        // 检查目录是否存在
        if !external_apps_dir.exists() {
            return Err("截屏程序目录不存在，请重新安装QuickClipboard".to_string());
        }
        
        let screenshot_exe = external_apps_dir.join("QCScreenshot.exe");
        
        if !screenshot_exe.exists() {
            return Err("截屏程序未找到或被删除\n\n请将QuickClipboardScreenshot.exe文件放入external_apps文件夹中".to_string());
        }
        
        println!("启动外部截屏程序: {}", screenshot_exe.display());
        
        // 启动外部截屏程序
        let mut command = Command::new(&screenshot_exe);
        command.current_dir(&external_apps_dir);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        
        match command.spawn() {
            Ok(mut child) => {
                println!("外部截屏程序已启动，PID: {:?}", child.id());
                
                // 读取子程序的输出来获取端口信息
                if let Some(stdout) = child.stdout.take() {
                    let reader = BufReader::new(stdout);
                    let app_handle_clone = app.clone();
                    
                    // 在新线程中读取输出，避免阻塞
                    std::thread::spawn(move || {
                        for line in reader.lines() {
                            if let Ok(line) = line {
                                println!("子程序输出: {}", line);
                                
                                // 解析端口信息，匹配子程序输出格式如 "QCScreenshot started on port: 8080"
                                if line.contains("QCScreenshot started on port:") {
                                    if let Some(port_str) = line.split(':').last() {
                                        if let Ok(port) = port_str.trim().parse::<u16>() {
                                            crate::screenshot_service::set_screenshot_service_port_and_start_heartbeat(port, app_handle_clone.clone());
                                            println!("成功解析到端口: {}，心跳检测服务已启动", port);
                                            break;
                                        }
                                    }
                                }
                                
                                // 也支持纯数字格式的端口输出
                                if let Ok(port) = line.trim().parse::<u16>() {
                                    if port > 1024 && port < 65535 {
                                        crate::screenshot_service::set_screenshot_service_port_and_start_heartbeat(port, app_handle_clone.clone());
                                        println!("成功解析到端口: {}，心跳检测服务已启动", port);
                                        break;
                                    }
                                }
                            }
                        }
                    });
                }
                
                // 立即detach，不等待进程结束
                std::mem::forget(child);
                Ok(())
            }
            Err(e) => {
                eprintln!("启动截屏程序失败: {}", e);
                Err(format!("启动截屏程序失败: {}", e))
            }
        }
    }
}
