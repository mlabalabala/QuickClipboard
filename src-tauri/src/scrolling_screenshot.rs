use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::{HWND, POINT as WIN_POINT};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BitBlt, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use serde::{Deserialize, Serialize};
use image::RgbaImage;

use crate::image_stitcher::{ImageStitcher, CapturedFrame};

const VERTICAL_PADDING: u32 = 40;

#[derive(Debug, Clone, Copy, PartialEq)]
enum ScrollingState {
    Idle,
    Running,
    Paused,
    Stopped,
}

pub struct ScrollingScreenshotManager {
    state: Arc<Mutex<ScrollingState>>,
    is_active: Arc<AtomicBool>,
    captured_frames: Arc<Mutex<Vec<CapturedFrame>>>,
    selection: Arc<Mutex<Option<SelectionRect>>>,
    panel_rect: Arc<Mutex<Option<PanelRect>>>,
    app_handle: Arc<Mutex<Option<AppHandle>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelRect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl ScrollingScreenshotManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ScrollingState::Idle)),
            is_active: Arc::new(AtomicBool::new(false)),
            captured_frames: Arc::new(Mutex::new(Vec::new())),
            selection: Arc::new(Mutex::new(None)),
            panel_rect: Arc::new(Mutex::new(None)),
            app_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub fn init(&self, app: AppHandle, selection: SelectionRect, panel: PanelRect) -> Result<(), String> {
        if self.is_active.load(Ordering::Relaxed) {
            return Err("长截屏已在运行中".to_string());
        }

        let scale_factor = app.get_webview_window("screenshot")
            .and_then(|w| w.scale_factor().ok())
            .unwrap_or(1.0);

        let physical_selection = SelectionRect {
            left: (selection.left as f64 * scale_factor) as i32,
            top: (selection.top as f64 * scale_factor) as i32,
            width: (selection.width as f64 * scale_factor) as i32,
            height: (selection.height as f64 * scale_factor) as i32,
        };

        *self.app_handle.lock().unwrap() = Some(app.clone());
        *self.selection.lock().unwrap() = Some(physical_selection);
        *self.state.lock().unwrap() = ScrollingState::Idle;
        self.captured_frames.lock().unwrap().clear();
        self.is_active.store(true, Ordering::Relaxed);
        
        // 使用统一的方法设置面板区域
        self.update_panel_rect(panel)?;
        
        // 立即设置初始穿透状态（面板外穿透）
        if let Some(window) = app.get_webview_window("screenshot") {
            let _ = window.set_ignore_cursor_events(true);
        }
        
        self.start_mouse_listener();

        Ok(())
    }

    pub fn start(&self) -> Result<(), String> {
        if !self.is_active.load(Ordering::Relaxed) {
            return Err("未初始化长截屏".to_string());
        }

        let mut state = self.state.lock().unwrap();
        if *state == ScrollingState::Running {
            return Err("长截屏已在运行中".to_string());
        }

        *state = ScrollingState::Running;
        drop(state);
        self.start_capture_thread();

        Ok(())
    }

    pub fn pause(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if *state != ScrollingState::Running {
            return Err("长截屏未在运行中".to_string());
        }
        *state = ScrollingState::Paused;
        Ok(())
    }

    pub fn resume(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if *state != ScrollingState::Paused {
            return Err("长截屏未暂停".to_string());
        }
        *state = ScrollingState::Running;
        Ok(())
    }

    pub fn stop(&self) -> Result<ScrollingResult, String> {
        *self.state.lock().unwrap() = ScrollingState::Stopped;
        self.is_active.store(false, Ordering::Relaxed);
        thread::sleep(Duration::from_millis(100));

        let result = self.merge_frames()?;
        self.save_to_clipboard(&result)?;
        self.cleanup();

        Ok(result)
    }

    pub fn cancel(&self) -> Result<(), String> {
        *self.state.lock().unwrap() = ScrollingState::Stopped;
        self.is_active.store(false, Ordering::Relaxed);
        self.cleanup();
        Ok(())
    }

    fn save_to_clipboard(&self, result: &ScrollingResult) -> Result<(), String> {
        let frames = self.captured_frames.lock().unwrap();
        if frames.is_empty() {
            return Err("没有捕获到任何帧".to_string());
        }

        let width = result.width;
        let height = result.height;
        let mut merged_data = vec![0u8; (width * height * 4) as usize];
        let mut current_y = 0u32;
        
        for frame in frames.iter() {
            let start_idx = (current_y * width * 4) as usize;
            let frame_size = (frame.width * frame.height * 4) as usize;
            if start_idx + frame_size <= merged_data.len() {
                merged_data[start_idx..start_idx + frame_size].copy_from_slice(&frame.data);
            }
            current_y += frame.height;
        }
        
        let png_bytes = ImageStitcher::bgra_to_png(&merged_data, width, height);
        
        // 保存到文件
        let app_data_dir = crate::settings::get_data_directory()?;
        let images_dir = app_data_dir.join("clipboard_images");
        // 创建专门的长截屏子目录
        let scrolling_dir = images_dir.join("scrolling_screenshots");
        std::fs::create_dir_all(&scrolling_dir)
            .map_err(|e| format!("创建长截屏目录失败: {}", e))?;
        
        // 生成唯一文件名（使用时间戳+毫秒）
        let now = chrono::Local::now();
        let timestamp = now.format("%Y%m%d_%H%M%S").to_string();
        let millis = now.timestamp_subsec_millis();
        let filename = format!("QC长截屏_{}_{:03}.png", timestamp, millis);
        let file_path = scrolling_dir.join(&filename);
        
        // 保存PNG文件
        std::fs::write(&file_path, &png_bytes)
            .map_err(|e| format!("保存图片文件失败: {}", e))?;
        
        println!("长截屏已保存到: {:?}", file_path);
        
        // 将文件路径复制到剪贴板
        let file_path_str = file_path.to_string_lossy().to_string();
        crate::file_handler::set_clipboard_files(&[file_path_str])?;
        
        Ok(())
    }

    fn start_mouse_listener(&self) {
        let is_active = Arc::clone(&self.is_active);
        let panel_rect = Arc::clone(&self.panel_rect);
        let app_handle = Arc::clone(&self.app_handle);
        
        thread::spawn(move || {
            let mut last_in_panel = false;
            
            while is_active.load(Ordering::Relaxed) {
                let mut cursor_pos = WIN_POINT { x: 0, y: 0 };
                if unsafe { GetCursorPos(&mut cursor_pos) }.is_err() {
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }

                if let Some(panel) = panel_rect.lock().unwrap().clone() {
                    let in_panel = cursor_pos.x >= panel.left 
                        && cursor_pos.x <= panel.left + panel.width
                        && cursor_pos.y >= panel.top 
                        && cursor_pos.y <= panel.top + panel.height;

                    if in_panel != last_in_panel {
                        last_in_panel = in_panel;
                        if let Some(app) = app_handle.lock().unwrap().as_ref() {
                            if let Some(window) = app.get_webview_window("screenshot") {
                                let _ = window.set_ignore_cursor_events(!in_panel);
                            }
                        }
                    }
                }

                thread::sleep(Duration::from_millis(50));
            }
            
            if let Some(app) = app_handle.lock().unwrap().as_ref() {
                if let Some(window) = app.get_webview_window("screenshot") {
                    let _ = window.set_ignore_cursor_events(false);
                }
            }
        });
    }

    fn start_capture_thread(&self) {
        let state = Arc::clone(&self.state);
        let is_active = Arc::clone(&self.is_active);
        let captured_frames = Arc::clone(&self.captured_frames);
        let selection = Arc::clone(&self.selection);
        let app_handle = Arc::clone(&self.app_handle);

        thread::spawn(move || {
            let mut total_height = 0;
            let mut no_change_count = 0;
            let mut last_extended_rgba: Option<RgbaImage> = None;
            let mut last_content_height: u32 = 0;
            let mut last_preview_time = std::time::Instant::now();


            loop {
                let current_state = *state.lock().unwrap();
                if current_state == ScrollingState::Stopped || !is_active.load(Ordering::Relaxed) {
                    break;
                }

                if current_state == ScrollingState::Paused {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }

                let sel = selection.lock().unwrap().clone();
                
                if let Some(sel) = sel {
                    let scale_factor = app_handle.lock().unwrap().as_ref()
                        .and_then(|app| app.get_webview_window("screenshot"))
                        .and_then(|w| w.scale_factor().ok())
                        .unwrap_or(1.0);
                    
                    let border_offset = (3.0 * scale_factor) as i32;
                    let content_left = sel.left + border_offset;
                    let content_top = sel.top + border_offset;
                    let content_width = sel.width - (border_offset * 2);
                    let content_height = sel.height - (border_offset * 2);

                    // 扩展截屏区域
                    let extended_top = content_top.saturating_sub(VERTICAL_PADDING as i32);
                    let extended_height = content_height + (VERTICAL_PADDING * 2) as i32;
                    
                    match Self::capture_region(content_left, extended_top, content_width, extended_height) {
                        Ok(frame_data) => {
                            let current_extended_rgba = ImageStitcher::bgra_to_rgba_image(&frame_data, content_width as u32, extended_height as u32);
                            let mut should_update_preview = false;
                            let mut is_first_frame = false;
                            
                            {
                                let mut frames_lock = captured_frames.lock().unwrap();
                                if frames_lock.is_empty() {
                                    // 第一帧：只截取实际内容区域
                                    let first_frame_data = ImageStitcher::extract_region(
                                        &frame_data,
                                        content_width as u32,
                                        VERTICAL_PADDING,
                                        content_height as u32,
                                    );
                                    frames_lock.push(CapturedFrame {
                                        data: first_frame_data,
                                        width: content_width as u32,
                                        height: content_height as u32,
                                    });
                                    total_height = content_height as u32;
                                    last_extended_rgba = Some(current_extended_rgba);
                                    last_content_height = content_height as u32;
                                    should_update_preview = true;
                                    is_first_frame = true;

                                } else if let Some(last_rgba) = &last_extended_rgba {
                                    // 后续帧：使用扩展区域进行匹配
                                    if let Some(stitch_result) = ImageStitcher::should_stitch_frame_ex(
                                        &last_rgba, &current_extended_rgba,
                                        VERTICAL_PADDING, last_content_height,
                                        VERTICAL_PADDING, content_height as u32
                                    ) {
                                        no_change_count = 0;
                                        
                                        let new_data = ImageStitcher::extract_region(
                                            &frame_data, 
                                            content_width as u32, 
                                            stitch_result.new_content_y, 
                                            stitch_result.new_content_height
                                        );
                                        
                                        frames_lock.push(CapturedFrame {
                                            data: new_data,
                                            width: content_width as u32,
                                            height: stitch_result.new_content_height,
                                        });
                                        
                                        total_height += stitch_result.new_content_height;
                                        last_extended_rgba = Some(current_extended_rgba);
                                        last_content_height = content_height as u32;
                                        should_update_preview = true;
                                    } else {
                                        no_change_count += 1;
                                    }
                                }
                            }
                            
                            // 实时预览更新
                            if should_update_preview {
                                let now = std::time::Instant::now();
                                let elapsed = now.duration_since(last_preview_time);
                                
                                // 首帧强制发送
                                if is_first_frame || elapsed >= Duration::from_millis(100) {
                                    last_preview_time = now;
                                    
                                    let app_clone = app_handle.lock().unwrap().as_ref().map(|a| a.clone());
                                    let frames_clone = captured_frames.lock().unwrap().clone();
                                    let total_height_clone = total_height;
                                    
                                    // 在新线程中快速处理预览
                                    thread::spawn(move || {
                                        if let Some(app) = app_clone {
                                            // 快速预览生成
                                            let preview_data = ImageStitcher::create_preview(&frames_clone);
                                            let bmp_data = Self::create_bmp_from_bgra(&preview_data.data, preview_data.width, preview_data.height);
                                            if let Ok(preview_url) = Self::serve_image_via_http(&bmp_data, preview_data.width, preview_data.height) {
                                                let _ = app.emit("scrolling-screenshot-preview", serde_json::json!({
                                                    "image_url": preview_url,
                                                    "height": total_height_clone,
                                                    "frames": frames_clone.len(),
                                                }));
                                            }
                                        }
                                    });
                                }
                            }

                            // 高速捕获模式：根据拼接成功率动态调整
                            if no_change_count > 20 {
                                // 很久没拼接成功，降低频率节省资源
                                thread::sleep(Duration::from_millis(60));
                            } else if no_change_count > 10 {
                                // 中等频率
                                thread::sleep(Duration::from_millis(35));
                            } else {
                                // 高频捕获，快速响应用户滚动
                                thread::sleep(Duration::from_millis(25));
                            }
                        }
                        Err(_) => {
                            thread::sleep(Duration::from_millis(80));
                        }
                    }
                }
            }
        });
    }

    fn capture_region(x: i32, y: i32, width: i32, height: i32) -> Result<Vec<u8>, String> {
        unsafe {
            let desktop_dc = GetDC(HWND(0));
            if desktop_dc.is_invalid() {
                return Err("获取桌面DC失败".to_string());
            }

            let mem_dc = CreateCompatibleDC(desktop_dc);
            if mem_dc.is_invalid() {
                let _ = ReleaseDC(HWND(0), desktop_dc);
                return Err("创建兼容DC失败".to_string());
            }

            let bitmap = CreateCompatibleBitmap(desktop_dc, width, height);
            if bitmap.is_invalid() {
                let _ = DeleteDC(mem_dc);
                let _ = ReleaseDC(HWND(0), desktop_dc);
                return Err("创建位图失败".to_string());
            }

            let _old_bitmap = SelectObject(mem_dc, bitmap);
            let _ = BitBlt(mem_dc, 0, 0, width, height, desktop_dc, x, y, SRCCOPY);

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

            let mut pixel_data = vec![0u8; (width * height * 4) as usize];
            let _ = GetDIBits(
                mem_dc,
                bitmap,
                0,
                height as u32,
                Some(pixel_data.as_mut_ptr() as *mut _),
                &mut bitmap_info,
                DIB_RGB_COLORS,
            );

            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(mem_dc);
            let _ = ReleaseDC(HWND(0), desktop_dc);

            Ok(pixel_data)
        }
    }

    fn merge_frames(&self) -> Result<ScrollingResult, String> {
        let frames = self.captured_frames.lock().unwrap();
        if frames.is_empty() {
            return Err("没有捕获到任何帧".to_string());
        }

        let width = frames[0].width;
        let total_height: u32 = frames.iter().map(|f| f.height).sum();
        let mut merged_data = vec![0u8; (width * total_height * 4) as usize];

        let mut current_y = 0;
        for frame in frames.iter() {
            for y in 0..frame.height as usize {
                let src_offset = y * (frame.width * 4) as usize;
                let dst_offset = ((current_y + y) * width as usize * 4) as usize;
                let row_size = (frame.width * 4) as usize;
                if dst_offset + row_size <= merged_data.len() && src_offset + row_size <= frame.data.len() {
                    merged_data[dst_offset..dst_offset + row_size].copy_from_slice(&frame.data[src_offset..src_offset + row_size]);
                }
            }
            current_y += frame.height as usize;
        }

        let bmp_data = Self::create_bmp_from_bgra(&merged_data, width, total_height);
        let image_url = Self::serve_image_via_http(&bmp_data, width, total_height)?;

        Ok(ScrollingResult {
            image_url,
            width,
            height: total_height,
            frames: frames.len(),
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

    fn serve_image_via_http(bmp_data: &[u8], _width: u32, _height: u32) -> Result<String, String> {
        use std::net::TcpListener;
        
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("绑定端口失败: {}", e))?;
        let port = listener.local_addr().unwrap().port();
        let image_data = Arc::new(bmp_data.to_vec());

        thread::spawn(move || {
            let start_time = std::time::Instant::now();
            while start_time.elapsed() < Duration::from_secs(10) {
                if let Ok((stream, _)) = listener.accept() {
                    Self::handle_http_request(stream, &image_data);
                }
            }
        });
        
        thread::sleep(Duration::from_millis(50));
        let url = format!("http://127.0.0.1:{}/scrolling.bmp?t={}", port, 
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
        Ok(url)
    }

    fn handle_http_request(stream: std::net::TcpStream, image_data: &[u8]) {
        use std::io::{Read, Write};
        
        let mut stream = stream;
        let mut buffer = [0; 1024];
        let _ = stream.read(&mut buffer);

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: image/bmp\r\nContent-Length: {}\r\n\
            Access-Control-Allow-Origin: *\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
            image_data.len()
        );

        let _ = stream.write_all(response.as_bytes());
        let _ = stream.write_all(image_data);
        let _ = stream.flush();
    }

    pub fn update_panel_rect(&self, panel: PanelRect) -> Result<(), String> {
        let scale_factor = self.app_handle.lock().unwrap().as_ref()
            .and_then(|app| app.get_webview_window("screenshot"))
            .and_then(|w| w.scale_factor().ok())
            .unwrap_or(1.0);

        let physical_panel = PanelRect {
            left: (panel.left as f64 * scale_factor) as i32,
            top: (panel.top as f64 * scale_factor) as i32,
            width: (panel.width as f64 * scale_factor) as i32,
            height: (panel.height as f64 * scale_factor) as i32,
        };

        // 更新面板区域后，立即检查并更新穿透状态
        let mut cursor_pos = WIN_POINT { x: 0, y: 0 };
        if unsafe { GetCursorPos(&mut cursor_pos) }.is_ok() {
            let in_panel = cursor_pos.x >= physical_panel.left 
                && cursor_pos.x <= physical_panel.left + physical_panel.width
                && cursor_pos.y >= physical_panel.top 
                && cursor_pos.y <= physical_panel.top + physical_panel.height;
            
            if let Some(app) = self.app_handle.lock().unwrap().as_ref() {
                if let Some(window) = app.get_webview_window("screenshot") {
                    let _ = window.set_ignore_cursor_events(!in_panel);
                }
            }
        }
        
        *self.panel_rect.lock().unwrap() = Some(physical_panel);
        
        Ok(())
    }

    fn cleanup(&self) {
        self.captured_frames.lock().unwrap().clear();
        *self.selection.lock().unwrap() = None;
        *self.panel_rect.lock().unwrap() = None;
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScrollingResult {
    pub image_url: String,
    pub width: u32,
    pub height: u32,
    pub frames: usize,
}

lazy_static::lazy_static! {
    pub static ref SCROLLING_SCREENSHOT_MANAGER: ScrollingScreenshotManager = ScrollingScreenshotManager::new();
}

#[tauri::command]
pub fn init_scrolling_screenshot(app: AppHandle, selection: SelectionRect, panel: PanelRect) -> Result<(), String> {
    SCROLLING_SCREENSHOT_MANAGER.init(app, selection, panel)
}

#[tauri::command]
pub fn start_scrolling_screenshot() -> Result<(), String> {
    SCROLLING_SCREENSHOT_MANAGER.start()
}

#[tauri::command]
pub fn pause_scrolling_screenshot() -> Result<(), String> {
    SCROLLING_SCREENSHOT_MANAGER.pause()
}

#[tauri::command]
pub fn resume_scrolling_screenshot() -> Result<(), String> {
    SCROLLING_SCREENSHOT_MANAGER.resume()
}

#[tauri::command]
pub fn stop_scrolling_screenshot() -> Result<ScrollingResult, String> {
    SCROLLING_SCREENSHOT_MANAGER.stop()
}

#[tauri::command]
pub fn cancel_scrolling_screenshot() -> Result<(), String> {
    SCROLLING_SCREENSHOT_MANAGER.cancel()
}

#[tauri::command]
pub fn update_scrolling_panel_rect(panel: PanelRect) -> Result<(), String> {
    SCROLLING_SCREENSHOT_MANAGER.update_panel_rect(panel)
}
