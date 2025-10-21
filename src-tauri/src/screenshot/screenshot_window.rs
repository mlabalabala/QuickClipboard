use windows::Win32::Graphics::Gdi::{GetDC, CreateCompatibleDC, CreateCompatibleBitmap, SelectObject, BitBlt, GetDIBits, DeleteDC, ReleaseDC, DeleteObject, BITMAPINFOHEADER, BITMAPINFO, BI_RGB, DIB_RGB_COLORS, SRCCOPY};
use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager};
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::sync::Arc;
use serde_json;

static SCREENSHOT_WINDOW_VISIBLE: AtomicBool = AtomicBool::new(false);

pub struct ScreenshotWindowManager;

impl ScreenshotWindowManager {
    pub fn show_screenshot_window(app: &tauri::AppHandle) -> Result<(), String> {
        let screenshot_window = app
            .get_webview_window("screenshot")
            .ok_or_else(|| "截屏窗口未找到".to_string())?;

        let _ = Self::set_fullscreen_size(app, &screenshot_window);

        let capture = Self::capture_screenshot_sync(&screenshot_window)
            .map_err(|e| format!("截屏失败: {}", e))?;
        let capture_width = capture.width;
        let capture_height = capture.height;
        let bmp_data = capture.data;

        screenshot_window
            .show()
            .map_err(|e| format!("显示截屏窗口失败: {}", e))?;

        screenshot_window
            .set_focus()
            .map_err(|e| format!("设置截屏窗口焦点失败: {}", e))?;

        SCREENSHOT_WINDOW_VISIBLE.store(true, Ordering::Relaxed);

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

    pub fn hide_screenshot_window(app: &tauri::AppHandle) -> Result<(), String> {
        let screenshot_window = app
            .get_webview_window("screenshot")
            .ok_or_else(|| "截屏窗口未找到".to_string())?;

        // 移出屏幕外避免Windows隐藏动画
        use tauri::PhysicalPosition;
        let _ = screenshot_window.set_position(PhysicalPosition::new(-10000, -10000));
        
        screenshot_window
            .hide()
            .map_err(|e| format!("隐藏截屏窗口失败: {}", e))?;

        SCREENSHOT_WINDOW_VISIBLE.store(false, Ordering::Relaxed);

        // 清除自动选区缓存
        super::auto_selection::AUTO_SELECTION_MANAGER.clear_cache();

        // 重载
        let _ = screenshot_window.eval("window.location.reload()");

        Ok(())
    }

    pub fn toggle_screenshot_window(app: &tauri::AppHandle) -> Result<(), String> {
        if SCREENSHOT_WINDOW_VISIBLE.load(Ordering::Relaxed) {
            Self::hide_screenshot_window(app)
        } else {
            Self::show_screenshot_window(app)
        }
    }

    pub fn is_screenshot_window_visible() -> bool {
        SCREENSHOT_WINDOW_VISIBLE.load(Ordering::Relaxed)
    }

    fn set_fullscreen_size(
        _app: &tauri::AppHandle,
        window: &tauri::WebviewWindow,
    ) -> Result<(), String> {
        use tauri::PhysicalPosition;
        use tauri::PhysicalSize;

        let (x, y, width, height) = super::screen_utils::ScreenUtils::get_virtual_screen_size_from_window(window)?;
        
        window
            .set_size(PhysicalSize::new(width as u32, height as u32))
            .map_err(|e| format!("设置窗口尺寸失败: {}", e))?;

        window
            .set_position(PhysicalPosition::new(x, y))
            .map_err(|e| format!("设置窗口位置失败: {}", e))?;

        Ok(())
    }

    pub fn get_all_monitors(window: &tauri::WebviewWindow) -> Result<Vec<super::screen_utils::MonitorInfo>, String> {
        super::screen_utils::ScreenUtils::get_all_monitors_from_window(window)
    }

    pub fn init_screenshot_window(app: &tauri::AppHandle) -> Result<(), String> {
        let screenshot_window = app
            .get_webview_window("screenshot")
            .ok_or_else(|| "截屏窗口未找到".to_string())?;

        let _ = screenshot_window.hide();
        SCREENSHOT_WINDOW_VISIBLE.store(false, Ordering::Relaxed);

        let screenshot_window_clone = screenshot_window.clone();
        screenshot_window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = screenshot_window_clone.hide();
                SCREENSHOT_WINDOW_VISIBLE.store(false, Ordering::Relaxed);
            }
        });

        Ok(())
    }
}

#[tauri::command]
pub fn get_css_monitors(
    window: tauri::WebviewWindow,
) -> Result<Vec<super::screen_utils::CssMonitorInfo>, String> {
    super::screen_utils::ScreenUtils::get_css_monitors(&window)
}

#[tauri::command]
pub fn constrain_selection_bounds(
    window: tauri::WebviewWindow,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(f64, f64), String> {
    let scale_factor = window.scale_factor().unwrap_or(1.0);

    let physical_x = (x * scale_factor).round() as i32;
    let physical_y = (y * scale_factor).round() as i32;
    let physical_width = (width * scale_factor).round() as i32;
    let physical_height = (height * scale_factor).round() as i32;

    let (constrained_physical_x, constrained_physical_y) =
        super::screen_utils::ScreenUtils::constrain_to_physical_bounds(
            physical_x,
            physical_y,
            physical_width,
            physical_height,
            &window,
        )?;

    let constrained_x = constrained_physical_x as f64 / scale_factor;
    let constrained_y = constrained_physical_y as f64 / scale_factor;

    Ok((constrained_x, constrained_y))
}

#[tauri::command]
pub fn show_screenshot_window(app: tauri::AppHandle) -> Result<(), String> {
    if SCREENSHOT_WINDOW_VISIBLE.load(Ordering::Relaxed) {
        return Ok(());
    }
    ScreenshotWindowManager::show_screenshot_window(&app)
}

#[tauri::command]
pub fn hide_screenshot_window(app: tauri::AppHandle) -> Result<(), String> {
    ScreenshotWindowManager::hide_screenshot_window(&app)
}

#[tauri::command]
pub fn toggle_screenshot_window(app: tauri::AppHandle) -> Result<(), String> {
    ScreenshotWindowManager::toggle_screenshot_window(&app)
}

#[tauri::command]
pub fn is_screenshot_window_visible() -> bool {
    ScreenshotWindowManager::is_screenshot_window_visible()
}

#[tauri::command]
pub fn get_all_monitors(window: tauri::WebviewWindow) -> Result<Vec<super::screen_utils::MonitorInfo>, String> {
    ScreenshotWindowManager::get_all_monitors(&window)
}

/// 设置鼠标到指定的物理像素位置（用于方向键精确移动）
#[tauri::command]
pub fn set_cursor_position_physical(x: i32, y: i32) -> Result<(), String> {
    crate::mouse_utils::set_cursor_position(x, y)
}

pub struct ScreenshotCapture {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl ScreenshotWindowManager {
    fn capture_screenshot_sync(window: &tauri::WebviewWindow) -> Result<ScreenshotCapture, String> {
        let (x, y, w, h) = super::screen_utils::ScreenUtils::get_virtual_screen_size_from_window(window)?;
        unsafe { Self::capture_with_gdi(x, y, w, h) }
    }

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

    fn create_bmp_from_bgra(pixel_data: &[u8], width: u32, height: u32) -> Vec<u8> {
        let pixel_data_size = pixel_data.len() as u32;
        let file_size = 54 + pixel_data_size;
        
        let mut bmp_data = Vec::with_capacity(file_size as usize);
        
        bmp_data.extend_from_slice(b"BM");
        bmp_data.extend_from_slice(&file_size.to_le_bytes());
        bmp_data.extend_from_slice(&0u16.to_le_bytes());
        bmp_data.extend_from_slice(&0u16.to_le_bytes());
        bmp_data.extend_from_slice(&54u32.to_le_bytes());
        
        bmp_data.extend_from_slice(&40u32.to_le_bytes());
        bmp_data.extend_from_slice(&width.to_le_bytes());
        bmp_data.extend_from_slice(&(-(height as i32)).to_le_bytes());
        bmp_data.extend_from_slice(&1u16.to_le_bytes());
        bmp_data.extend_from_slice(&32u16.to_le_bytes());
        bmp_data.extend_from_slice(&0u32.to_le_bytes());
        bmp_data.extend_from_slice(&pixel_data_size.to_le_bytes());
        bmp_data.extend_from_slice(&0u32.to_le_bytes());
        bmp_data.extend_from_slice(&0u32.to_le_bytes());
        bmp_data.extend_from_slice(&0u32.to_le_bytes());
        bmp_data.extend_from_slice(&0u32.to_le_bytes());
        
        bmp_data.extend_from_slice(pixel_data);
        
        bmp_data
    }

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
    
    fn handle_http_request(mut stream: TcpStream, image_data: &[u8]) {
        use std::io::Read;
        
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer);
        
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
            Content-Type: image/bmp\r\n\
            Content-Length: {}\r\n\
            Access-Control-Allow-Origin: *\r\n\
            Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n\
            Access-Control-Allow-Headers: *\r\n\
            Cache-Control: no-cache\r\n\
            Connection: close\r\n\
            \r\n",
            image_data.len()
        );
        
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.write_all(image_data);
        let _ = stream.flush();
    }

}
