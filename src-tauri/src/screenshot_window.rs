use windows::core::Interface;
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, IDXGIAdapter1, IDXGIOutput, IDXGIOutput1};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING, D3D11_CPU_ACCESS_READ,
    D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ
};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Gdi::{GetDC, CreateCompatibleDC, CreateCompatibleBitmap, SelectObject, BitBlt, GetDIBits, DeleteDC, ReleaseDC, DeleteObject, BITMAPINFOHEADER, BITMAPINFO, BI_RGB, DIB_RGB_COLORS, SRCCOPY};
use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;
use windows::Win32::System::Com::{CoInitialize, CoUninitialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::sync::Arc;
use serde_json;

/// 截屏窗口状态管理
static SCREENSHOT_WINDOW_VISIBLE: AtomicBool = AtomicBool::new(false);

/// 截屏窗口管理器
pub struct ScreenshotWindowManager;

impl ScreenshotWindowManager {
    /// 显示截屏窗口
    pub fn show_screenshot_window(app: &tauri::AppHandle) -> Result<(), String> {
        // 获取截屏窗口
        let screenshot_window = app
            .get_webview_window("screenshot")
            .ok_or_else(|| "截屏窗口未找到".to_string())?;

        // 获取屏幕尺寸并设置窗口为全屏
        if let Err(_) = Self::set_fullscreen_size(app, &screenshot_window) {
            // 继续执行，使用默认尺寸
        }

        // 先截屏（完全同步，截屏完成后再显示窗口）
        let capture = Self::capture_screenshot_sync()
            .map_err(|e| format!("截屏失败: {}", e))?;
        let capture_width = capture.width;
        let capture_height = capture.height;
        let bmp_data = capture.data;

        // 截屏完成，现在可以安全显示窗口
        screenshot_window
            .show()
            .map_err(|e| format!("显示截屏窗口失败: {}", e))?;

        // 将窗口置顶并获得焦点
        screenshot_window
            .set_focus()
            .map_err(|e| format!("设置截屏窗口焦点失败: {}", e))?;

        // 更新窗口可见状态
        SCREENSHOT_WINDOW_VISIBLE.store(true, Ordering::Relaxed);

        // 本地HTTP服务器 + 浏览器原生加载（最快方案）
        let window_for_data = screenshot_window.clone();
        
        std::thread::spawn(move || {
            match Self::serve_screenshot_via_http(&bmp_data, capture_width, capture_height) {
                Ok(image_url) => {
                    let payload = serde_json::json!({
                        "width": capture_width,
                        "height": capture_height,
                        "image_url": image_url,
                    });
                    
                    let _ = window_for_data.emit("screenshot-ready", payload);
                },
                Err(_) => {
                    let _ = window_for_data.emit("screenshot-error", "HTTP服务器启动失败");
                }
            }
        });

        Ok(())
    }

    /// 隐藏截屏窗口
    pub fn hide_screenshot_window(app: &tauri::AppHandle) -> Result<(), String> {
        // 获取截屏窗口
        let screenshot_window = app
            .get_webview_window("screenshot")
            .ok_or_else(|| "截屏窗口未找到".to_string())?;

        // 隐藏窗口
        screenshot_window
            .hide()
            .map_err(|e| format!("隐藏截屏窗口失败: {}", e))?;

        // 更新窗口可见状态
        SCREENSHOT_WINDOW_VISIBLE.store(false, Ordering::Relaxed);

        Ok(())
    }

    /// 切换截屏窗口显示状态
    pub fn toggle_screenshot_window(app: &tauri::AppHandle) -> Result<(), String> {
        if SCREENSHOT_WINDOW_VISIBLE.load(Ordering::Relaxed) {
            Self::hide_screenshot_window(app)
        } else {
            Self::show_screenshot_window(app)
        }
    }

    /// 检查截屏窗口是否可见
    pub fn is_screenshot_window_visible() -> bool {
        SCREENSHOT_WINDOW_VISIBLE.load(Ordering::Relaxed)
    }

    /// 设置窗口为跨所有显示器的全屏尺寸
    fn set_fullscreen_size(
        _app: &tauri::AppHandle,
        window: &tauri::WebviewWindow,
    ) -> Result<(), String> {
        use tauri::LogicalPosition;
        use tauri::LogicalSize;

        // 获取缩放因子并使用统一工具函数转换
        let scale_factor = window.scale_factor().unwrap_or(1.0);
        let (logical_x, logical_y, logical_width, logical_height) =
            crate::screen_utils::ScreenUtils::get_css_virtual_screen_size(scale_factor)?;

        // 使用LogicalSize设置逻辑尺寸，让Tauri处理缩放
        let size = LogicalSize::new(logical_width, logical_height);
        window
            .set_size(size)
            .map_err(|e| format!("设置窗口尺寸失败: {}", e))?;

        // 使用LogicalPosition设置逻辑位置
        let position = LogicalPosition::new(logical_x, logical_y);
        window
            .set_position(position)
            .map_err(|e| format!("设置窗口位置失败: {}", e))?;

        Ok(())
    }


    /// 获取所有显示器信息（物理像素格式，供后端使用）
    pub fn get_all_monitors() -> Result<Vec<crate::screen_utils::MonitorInfo>, String> {
        crate::screen_utils::ScreenUtils::get_all_monitors()
    }

    /// 初始化截屏窗口
    pub fn init_screenshot_window(app: &tauri::AppHandle) -> Result<(), String> {
        // 获取截屏窗口
        let screenshot_window = app
            .get_webview_window("screenshot")
            .ok_or_else(|| "截屏窗口未找到".to_string())?;

        // 确保窗口初始状态为隐藏
        let _ = screenshot_window.hide();
        SCREENSHOT_WINDOW_VISIBLE.store(false, Ordering::Relaxed);

        // 设置窗口关闭事件处理 - 隐藏而不是关闭
        let screenshot_window_clone = screenshot_window.clone();
        screenshot_window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 阻止默认的关闭行为
                api.prevent_close();
                // 隐藏窗口
                let _ = screenshot_window_clone.hide();
                SCREENSHOT_WINDOW_VISIBLE.store(false, Ordering::Relaxed);
            }
        });

        Ok(())
    }
}

/// 获取所有显示器信息（返回CSS像素格式，供前端使用）
#[tauri::command]
pub fn get_css_monitors(
    window: tauri::WebviewWindow,
) -> Result<Vec<crate::screen_utils::CssMonitorInfo>, String> {
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    crate::screen_utils::ScreenUtils::get_css_monitors(scale_factor)
}

/// 约束选区位置到合适的显示器边界内（复用贴边隐藏的边界逻辑）
#[tauri::command]
pub fn constrain_selection_bounds(
    window: tauri::WebviewWindow,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(f64, f64), String> {
    let scale_factor = window.scale_factor().unwrap_or(1.0);

    // 转换为物理像素
    let physical_x = (x * scale_factor) as i32;
    let physical_y = (y * scale_factor) as i32;
    let physical_width = (width * scale_factor) as i32;
    let physical_height = (height * scale_factor) as i32;

    // 使用物理像素边界约束（复用窗口拖拽逻辑）
    let (constrained_physical_x, constrained_physical_y) =
        crate::screen_utils::ScreenUtils::constrain_to_physical_bounds(
            physical_x,
            physical_y,
            physical_width,
            physical_height,
            &window,
        )?;

    // 转换回CSS像素
    let constrained_x = constrained_physical_x as f64 / scale_factor;
    let constrained_y = constrained_physical_y as f64 / scale_factor;

    Ok((constrained_x, constrained_y))
}

/// 命令函数：显示截屏窗口
#[tauri::command]
pub fn show_screenshot_window(app: tauri::AppHandle) -> Result<(), String> {
    ScreenshotWindowManager::show_screenshot_window(&app)
}

/// 命令函数：隐藏截屏窗口
#[tauri::command]
pub fn hide_screenshot_window(app: tauri::AppHandle) -> Result<(), String> {
    ScreenshotWindowManager::hide_screenshot_window(&app)
}

/// 命令函数：切换截屏窗口显示状态
#[tauri::command]
pub fn toggle_screenshot_window(app: tauri::AppHandle) -> Result<(), String> {
    ScreenshotWindowManager::toggle_screenshot_window(&app)
}

/// 命令函数：检查截屏窗口是否可见
#[tauri::command]
pub fn is_screenshot_window_visible() -> bool {
    ScreenshotWindowManager::is_screenshot_window_visible()
}

/// 获取所有显示器信息的命令
#[tauri::command]
pub fn get_all_monitors() -> Result<Vec<crate::screen_utils::MonitorInfo>, String> {
    ScreenshotWindowManager::get_all_monitors()
}

pub struct ScreenshotCapture {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl ScreenshotWindowManager {
    /// DXGI高性能截屏，GDI回退（最快方案）
    fn capture_screenshot_sync() -> Result<ScreenshotCapture, String> {
        unsafe {
            let _ = CoInitialize(None);
            
            // 首先尝试DXGI（最快）
            let result = Self::capture_with_dxgi().or_else(|dxgi_err| {
                // DXGI失败，使用GDI回退
                eprintln!("DXGI截屏失败: {}, 尝试GDI回退", dxgi_err);
                let (x, y, w, h) = crate::screen_utils::ScreenUtils::get_virtual_screen_size()?;
                Self::capture_with_gdi(x, y, w, h)
            });
            
            CoUninitialize();
            result
        }
    }

    /// 使用DXGI进行截屏并封装为BMP
    unsafe fn capture_with_dxgi() -> Result<ScreenshotCapture, String> {
        // 创建DXGI工厂
        let factory: IDXGIFactory1 = CreateDXGIFactory1().map_err(|e| format!("创建DXGI工厂失败: {}", e))?;

        // 枚举适配器
        let adapter: IDXGIAdapter1 = factory.EnumAdapters1(0).map_err(|e| format!("枚举适配器失败: {}", e))?;

        // 创建D3D11设备
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;
        
        D3D11CreateDevice(
            &adapter,
            D3D_DRIVER_TYPE_UNKNOWN,
            None,
            D3D11_CREATE_DEVICE_FLAG(0),
            Some(&[D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context)
        ).map_err(|e| format!("创建D3D11设备失败: {}", e))?;

        let device = device.unwrap();
        let context = context.unwrap();

        // 获取输出
        let output: IDXGIOutput = adapter.EnumOutputs(0).map_err(|e| format!("枚举输出失败: {}", e))?;
        let output1: IDXGIOutput1 = output.cast().map_err(|e| format!("转换输出接口失败: {}", e))?;

        // 复制桌面
        let duplication = output1.DuplicateOutput(&device).map_err(|e| format!("创建桌面复制失败: {}", e))?;

        // 获取一帧
        let mut frame_info = Default::default();
        let mut desktop_resource = None;
        
        duplication.AcquireNextFrame(0, &mut frame_info, &mut desktop_resource)
            .map_err(|e| format!("获取桌面帧失败: {}", e))?;

        let desktop_texture: ID3D11Texture2D = desktop_resource.unwrap()
            .cast().map_err(|e| format!("转换桌面纹理失败: {}", e))?;

        // 获取纹理描述
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        desktop_texture.GetDesc(&mut desc);

        // 创建CPU可访问的纹理
        desc.Usage = D3D11_USAGE_STAGING;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        desc.BindFlags = 0;

        let mut cpu_texture: Option<ID3D11Texture2D> = None;
        device.CreateTexture2D(&desc, None, Some(&mut cpu_texture))
            .map_err(|e| format!("创建CPU纹理失败: {}", e))?;
        let cpu_texture = cpu_texture.unwrap();

        // 复制纹理数据
        context.CopyResource(&cpu_texture, &desktop_texture);

        // 映射纹理获取像素数据
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        context.Map(&cpu_texture, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
            .map_err(|e| format!("映射纹理失败: {}", e))?;

        // 复制像素数据
        let pixel_count = (desc.Width * desc.Height) as usize;
        let mut pixel_data: Vec<u8> = Vec::with_capacity(pixel_count * 4);

        let src_data = std::slice::from_raw_parts(
            mapped.pData as *const u8,
            mapped.RowPitch as usize * desc.Height as usize
        );

        for y in 0..desc.Height {
            let row_start = (y * mapped.RowPitch) as usize;
            let row_end = row_start + (desc.Width * 4) as usize;
            pixel_data.extend_from_slice(&src_data[row_start..row_end]);
        }

        // 取消映射
        context.Unmap(&cpu_texture, 0);

        // 释放帧
        duplication.ReleaseFrame().ok();

        // 封装为BMP格式
        let bmp_data = Self::create_bmp_from_bgra(&pixel_data, desc.Width, desc.Height);

        Ok(ScreenshotCapture {
            data: bmp_data,
            width: desc.Width,
            height: desc.Height,
        })
    }

    /// GDI回退截屏方案
    unsafe fn capture_with_gdi(x: i32, y: i32, width: i32, height: i32) -> Result<ScreenshotCapture, String> {
        let desktop_wnd = GetDesktopWindow();
        let desktop_dc = GetDC(desktop_wnd);
        if desktop_dc.is_invalid() {
            return Err("获取桌面DC失败".to_string());
        }

        let mem_dc = CreateCompatibleDC(desktop_dc);
        if mem_dc.is_invalid() {
            ReleaseDC(desktop_wnd, desktop_dc);
            return Err("创建兼容DC失败".to_string());
        }

        let bitmap = CreateCompatibleBitmap(desktop_dc, width, height);
        if bitmap.is_invalid() {
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(desktop_wnd, desktop_dc);
            return Err("创建位图失败".to_string());
        }

        let old_bitmap = SelectObject(mem_dc, bitmap);
        let success = BitBlt(mem_dc, 0, 0, width, height, desktop_dc, x, y, SRCCOPY);

        if success.is_err() {
            let _ = SelectObject(mem_dc, old_bitmap);
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(desktop_wnd, desktop_dc);
            return Err("截屏失败".to_string());
        }

        let mut bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default(); 1],
        };

        let pixel_count = (width * height) as usize;
        let mut pixel_data: Vec<u8> = vec![0; pixel_count * 4];

        let lines = GetDIBits(
            mem_dc,
            bitmap,
            0,
            height as u32,
            Some(pixel_data.as_mut_ptr() as *mut _),
            &mut bitmap_info,
            DIB_RGB_COLORS
        );

        // 清理资源
        let _ = SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(bitmap);
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(desktop_wnd, desktop_dc);

        if lines == 0 {
            return Err("获取位图数据失败".to_string());
        }

        let bmp_data = Self::create_bmp_from_bgra(&pixel_data, width as u32, height as u32);

        Ok(ScreenshotCapture {
            data: bmp_data,
            width: width as u32,
            height: height as u32,
        })
    }

    /// 创建BMP文件（直接从BGRA像素数据）
    fn create_bmp_from_bgra(pixel_data: &[u8], width: u32, height: u32) -> Vec<u8> {
        let pixel_data_size = pixel_data.len() as u32;
        let file_size = 54 + pixel_data_size; // 54字节BMP头 + 像素数据
        
        let mut bmp_data = Vec::with_capacity(file_size as usize);
        
        // BMP文件头 (14字节)
        bmp_data.extend_from_slice(b"BM");                    // 文件类型
        bmp_data.extend_from_slice(&file_size.to_le_bytes()); // 文件大小
        bmp_data.extend_from_slice(&0u16.to_le_bytes());      // 保留字段1
        bmp_data.extend_from_slice(&0u16.to_le_bytes());      // 保留字段2  
        bmp_data.extend_from_slice(&54u32.to_le_bytes());     // 像素数据偏移
        
        // BMP信息头 (40字节)
        bmp_data.extend_from_slice(&40u32.to_le_bytes());     // 信息头大小
        bmp_data.extend_from_slice(&width.to_le_bytes());     // 图像宽度
        bmp_data.extend_from_slice(&(-(height as i32)).to_le_bytes()); // 图像高度（负值=从上到下）
        bmp_data.extend_from_slice(&1u16.to_le_bytes());      // 颜色平面数
        bmp_data.extend_from_slice(&32u16.to_le_bytes());     // 每像素位数(32位BGRA)
        bmp_data.extend_from_slice(&0u32.to_le_bytes());      // 压缩方式(0=不压缩)
        bmp_data.extend_from_slice(&pixel_data_size.to_le_bytes()); // 图像数据大小
        bmp_data.extend_from_slice(&0u32.to_le_bytes());      // 水平分辨率
        bmp_data.extend_from_slice(&0u32.to_le_bytes());      // 垂直分辨率
        bmp_data.extend_from_slice(&0u32.to_le_bytes());      // 调色板颜色数
        bmp_data.extend_from_slice(&0u32.to_le_bytes());      // 重要颜色数
        
        // 直接添加BGRA像素数据（无需任何转换！）
        bmp_data.extend_from_slice(pixel_data);
        
        bmp_data
    }

    /// 通过本地HTTP服务器提供截图服务
    fn serve_screenshot_via_http(bmp_data: &[u8], _width: u32, _height: u32) -> Result<String, String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("绑定端口失败: {}", e))?;
        let port = listener.local_addr().unwrap().port();
        
        let image_data = Arc::new(bmp_data.to_vec());
        
        thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                Self::handle_http_request(stream, &image_data);
            }
        });
        
        Ok(format!("http://127.0.0.1:{}/screenshot.bmp", port))
    }
    
    /// 处理HTTP请求
    fn handle_http_request(mut stream: TcpStream, image_data: &[u8]) {
        use std::io::Read;
        
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer);
        
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Type: image/bmp\r\n\
            Content-Length: {}\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Connection: close\r\n\
            \r\n",
            image_data.len()
        );
        
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.write_all(image_data);
        let _ = stream.flush();
    }

}
