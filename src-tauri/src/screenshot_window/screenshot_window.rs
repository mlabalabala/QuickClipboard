use image::{DynamicImage, GenericImageView};
use screenshots::Screen;
use std::sync::{Arc, Mutex};
use tauri::Manager;

use super::screenshot_state::{
    NativeScreenshotState, NATIVE_SCREENSHOT_STATE, SHOULD_EXIT_MESSAGE_LOOP,
    cleanup_background_resources, reset_global_state
};
use super::screenshot_render::compose_and_present;
use super::screenshot_events::{
    handle_mouse_down, handle_mouse_move, handle_mouse_up, handle_confirm,
    handle_set_cursor, handle_right_button, WM_CONFIRM_SCREENSHOT
};

#[cfg(windows)]
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::{Dwm::*, Gdi::*},
        System::LibraryLoader::GetModuleHandleW,
        UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_RETURN},
        UI::WindowsAndMessaging::*,
    },
};

// 屏蔽本文件内的所有调试输出
#[allow(unused_macros)]
macro_rules! println {
    ($($arg:tt)*) => {};
}

/// 原生截屏窗口管理器
pub struct NativeScreenshotWindow {
    #[cfg(windows)]
    hwnd: Option<HWND>,
}

impl NativeScreenshotWindow {
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            hwnd: None,
        }
    }

    /// 开始原生截屏流程
    pub async fn start_screenshot(&mut self) -> std::result::Result<(), String> {
        // 重置退出标志
        *SHOULD_EXIT_MESSAGE_LOOP.lock().unwrap() = false;

        // 1. 截取全屏图像
        let (screenshot_image, screen_width, screen_height) = self.capture_fullscreen().await?;

        // 2. 保存到全局状态
        {
            let mut state = NATIVE_SCREENSHOT_STATE.lock().unwrap();
            *state = Some(Arc::new(NativeScreenshotState::new(
                screenshot_image,
                screen_width,
                screen_height,
            )));
        }

        // 3. 创建透明全屏窗口
        #[cfg(windows)]
        self.create_transparent_window()?;

        // 4. 创建双缓冲背景（改为生成基础 BGRA 缓冲）
        #[cfg(windows)]
        self.create_background_buffer()?;

        // 5. 确保窗口完全准备好
        #[cfg(windows)]
        unsafe {
            if let Some(hwnd) = self.hwnd {
                InvalidateRect(hwnd, None, FALSE);
                UpdateWindow(hwnd);
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }

        // 6. 开始消息循环处理用户交互
        #[cfg(windows)]
        {
            let result = self.run_message_loop();
            self.cleanup();
            if let Err(e) = result { return Err(e); }
        }

        Ok(())
    }

    /// 截取全屏图像
    async fn capture_fullscreen(
        &mut self,
    ) -> std::result::Result<(DynamicImage, i32, i32), String> {
        let screens = Screen::all().map_err(|e| format!("获取屏幕信息失败: {}", e))?;

        // 计算所有屏幕的总边界
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for screen in &screens {
            let display = &screen.display_info;
            min_x = min_x.min(display.x);
            min_y = min_y.min(display.y);
            max_x = max_x.max(display.x + display.width as i32);
            max_y = max_y.max(display.y + display.height as i32);
        }

        let total_width = (max_x - min_x) as u32;
        let total_height = (max_y - min_y) as u32;

        // 创建目标图像
        let mut target_image = image::ImageBuffer::new(total_width, total_height);

        // 截取每个屏幕并合成
        for screen in &screens {
            let display = &screen.display_info;

            match screen.capture() {
                Ok(screen_image) => {
                    let offset_x = (display.x - min_x) as u32;
                    let offset_y = (display.y - min_y) as u32;

                    for (src_x, src_y, pixel) in screen_image.enumerate_pixels() {
                        let dst_x = offset_x + src_x;
                        let dst_y = offset_y + src_y;
                        if dst_x < total_width && dst_y < total_height {
                            target_image.put_pixel(dst_x, dst_y, *pixel);
                        }
                    }
                }
                Err(e) => {
                    println!("截取屏幕失败: {}", e);
                }
            }
        }

        let screenshot_image = DynamicImage::ImageRgba8(target_image);
        Ok((screenshot_image, total_width as i32, total_height as i32))
    }

    /// 创建透明全屏窗口
    #[cfg(windows)]
    fn create_transparent_window(&mut self) -> std::result::Result<(), String> {
        unsafe {
            let instance = match GetModuleHandleW(None) { 
                Ok(h) => h, 
                Err(e) => return Err(format!("获取模块句柄失败: {}", e)) 
            };
            let class_name = w!("NativeScreenshotWindow");

            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_OWNDC,
                lpfnWndProc: Some(Self::window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance.into(),
                hIcon: HICON::default(),
                hCursor: LoadCursorW(None, IDC_CROSS).unwrap_or_default(),
                hbrBackground: HBRUSH(0),
                lpszMenuName: PCWSTR::null(),
                lpszClassName: class_name,
                hIconSm: HICON::default(),
            };

            let register_result = RegisterClassExW(&wc);
            if register_result == 0 { /* 类已存在也可继续 */ } else { /* 注册成功 */ }

            let virtual_screen_left = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let virtual_screen_top = GetSystemMetrics(SM_YVIRTUALSCREEN);
            let virtual_screen_width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let virtual_screen_height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                class_name,
                w!("Native Screenshot"),
                WS_POPUP,
                virtual_screen_left,
                virtual_screen_top,
                virtual_screen_width,
                virtual_screen_height,
                None,
                None,
                instance,
                Some(self as *mut _ as *mut _),
            );

            if hwnd.0 == 0 { return Err("创建窗口失败".to_string()); }

            self.hwnd = Some(hwnd);

            if let Err(e) = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA) {
                return Err(format!("设置窗口透明度失败: {}", e));
            }

            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
        }
        Ok(())
    }

    /// 创建双缓冲背景
    #[cfg(windows)]
    fn create_background_buffer(&mut self) -> std::result::Result<(), String> {
        unsafe {
            // 1) 生成基础 BGRA 预乘缓存（一次）
            let (width, height, base_bgra) = {
                let state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
                if let Some(ref state_arc) = *state_guard {
                    let img = &state_arc.screenshot_image;
                    let (w, h) = img.dimensions();
                    let mut bgra: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
                    let rgba = img.to_rgba8();
                    for p in rgba.pixels() {
                        let r = p[0] as u32;
                        let g = p[1] as u32;
                        let b = p[2] as u32;
                        let a = p[3] as u32;
                        let r_p = (r * a + 127) / 255;
                        let g_p = (g * a + 127) / 255;
                        let b_p = (b * a + 127) / 255;
                        bgra.push(b_p as u8);
                        bgra.push(g_p as u8);
                        bgra.push(r_p as u8);
                        bgra.push(a as u8);
                    }
                    (w as i32, h as i32, bgra)
                } else {
                    return Err("无法获取截屏图像".to_string());
                }
            };

            // 2) 创建内存 DC + DIBSection，写入一次背景像素
            if let Some(hwnd) = self.hwnd {
                let window_dc = GetDC(hwnd);
                if window_dc.is_invalid() { return Err("获取窗口DC失败".to_string()); }

                let memory_dc = CreateCompatibleDC(window_dc);
                if memory_dc.is_invalid() {
                    let _ = ReleaseDC(hwnd, window_dc);
                    return Err("创建内存DC失败".to_string());
                }

                let mut bmi = BITMAPINFO {
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
                    bmiColors: [RGBQUAD::default(); 1],
                };

                let mut bits_ptr: *mut core::ffi::c_void = std::ptr::null_mut();
                let dib = CreateDIBSection(
                    window_dc,
                    &mut bmi,
                    DIB_RGB_COLORS,
                    &mut bits_ptr,
                    None,
                    0,
                ).unwrap();

                let _old = SelectObject(memory_dc, dib);
                let dst_slice = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, (width * height * 4) as usize);
                dst_slice.copy_from_slice(&base_bgra);

                // 2.5) 创建 1x1 半透明黑色叠加源（用于 AlphaBlend 蒙版）
                let overlay_dc = CreateCompatibleDC(window_dc);
                if overlay_dc.is_invalid() {
                    let _ = DeleteObject(dib);
                    let _ = DeleteDC(memory_dc);
                    let _ = ReleaseDC(hwnd, window_dc);
                    return Err("创建overlay DC失败".to_string());
                }

                let mut overlay_bmi = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: 1,
                        biHeight: -1,
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB.0,
                        biSizeImage: 0,
                        biXPelsPerMeter: 0,
                        biYPelsPerMeter: 0,
                        biClrUsed: 0,
                        biClrImportant: 0,
                    },
                    bmiColors: [RGBQUAD::default(); 1],
                };
                let mut overlay_bits: *mut core::ffi::c_void = std::ptr::null_mut();
                let overlay_dib = CreateDIBSection(
                    window_dc,
                    &mut overlay_bmi,
                    DIB_RGB_COLORS,
                    &mut overlay_bits,
                    None,
                    0,
                ).unwrap();
                let _ = SelectObject(overlay_dc, overlay_dib);
                if !overlay_bits.is_null() { 
                    let px = std::slice::from_raw_parts_mut(overlay_bits as *mut u8, 4); 
                    px[0] = 0; px[1] = 0; px[2] = 0; px[3] = 120; 
                }

                // 3) 保存到全局状态
                {
                    let mut state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
                    if let Some(ref mut state_arc) = *state_guard {
                        if let Some(state) = Arc::get_mut(state_arc) {
                            state.base_bgra_premul = Some(base_bgra);
                            state.screen_width = width;
                            state.screen_height = height;
                            state.background_dc = Some(memory_dc);
                            state.background_bitmap = Some(dib);
                            state.overlay_dc = Some(overlay_dc);
                            state.overlay_bitmap = Some(overlay_dib);
                        }
                    }
                }

                // 2.6) 创建离屏合成缓冲（frame_dc）
                let frame_dc = CreateCompatibleDC(window_dc);
                if frame_dc.is_invalid() {
                    let _ = ReleaseDC(hwnd, window_dc);
                    return Err("创建frame DC失败".to_string());
                }
                let mut frame_bmi = BITMAPINFO {
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
                    bmiColors: [RGBQUAD::default(); 1],
                };
                let mut frame_bits: *mut core::ffi::c_void = std::ptr::null_mut();
                let frame_dib = CreateDIBSection(
                    window_dc,
                    &mut frame_bmi,
                    DIB_RGB_COLORS,
                    &mut frame_bits,
                    None,
                    0,
                ).unwrap();
                let _ = SelectObject(frame_dc, frame_dib);

                {
                    let mut state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
                    if let Some(ref mut state_arc) = *state_guard {
                        if let Some(state) = Arc::get_mut(state_arc) {
                            state.frame_dc = Some(frame_dc);
                            state.frame_bitmap = Some(frame_dib);
                        }
                    }
                }

                let _ = ReleaseDC(hwnd, window_dc);
                Ok(())
            } else { 
                Err("窗口句柄无效".to_string()) 
            }
        }
    }

    /// 窗口过程函数
    #[cfg(windows)]
    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_CREATE => { LRESULT(0) }
            WM_PAINT => { Self::handle_paint(hwnd); LRESULT(0) }
            WM_ERASEBKGND => { LRESULT(1) }
            WM_LBUTTONDOWN => { handle_mouse_down(hwnd, lparam); LRESULT(0) }
            WM_MOUSEMOVE => { handle_mouse_move(hwnd, lparam); LRESULT(0) }
            WM_LBUTTONUP => { handle_mouse_up(hwnd, lparam); LRESULT(0) }
            WM_SETCURSOR => { handle_set_cursor(hwnd); LRESULT(1) }
            // 右键：优先清空选区，无选区时取消截屏
            WM_RBUTTONDOWN => { handle_right_button(hwnd); LRESULT(0) }
            WM_KEYDOWN => {
                if wparam.0 == VK_ESCAPE.0 as usize { let _ = DestroyWindow(hwnd); }
                else if wparam.0 == VK_RETURN.0 as usize { handle_confirm(hwnd); }
                LRESULT(0)
            }
            WM_CONFIRM_SCREENSHOT => { handle_confirm(hwnd); LRESULT(0) }
            WM_DESTROY => { *SHOULD_EXIT_MESSAGE_LOOP.lock().unwrap() = true; LRESULT(0) }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    /// 处理绘制消息 - 背景层 BitBlt + 操作层（仅绘制叠加元素）
    #[cfg(windows)]
    unsafe fn handle_paint(hwnd: HWND) {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);

        let state_arc = { 
            let state = NATIVE_SCREENSHOT_STATE.lock().unwrap(); 
            state.clone() 
        };
        if let Some(state) = state_arc { 
            compose_and_present(hwnd, hdc, &state, ps.rcPaint); 
        } else {
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            let brush = CreateSolidBrush(COLORREF(0x00000000));
            let _ = FillRect(hdc, &rect, brush);
            let _ = DeleteObject(brush);
        }
        let _ = EndPaint(hwnd, &ps);
    }

    #[cfg(windows)]
    fn run_message_loop(&self) -> std::result::Result<(), String> {
        unsafe {
            let mut msg = MSG::default();
            loop {
                if *SHOULD_EXIT_MESSAGE_LOOP.lock().unwrap() { break; }
                let result = if let Some(hwnd) = self.hwnd { 
                    GetMessageW(&mut msg, hwnd, 0, 0) 
                } else { 
                    GetMessageW(&mut msg, None, 0, 0) 
                };
                if result.0 == 0 { break; }
                else if result.0 == -1 { return Err("消息循环错误".to_string()); }
                else { 
                    let _ = TranslateMessage(&msg); 
                    DispatchMessageW(&msg); 
                }
            }
        }
        Ok(())
    }

    pub fn cleanup(&mut self) {
        #[cfg(windows)]
        unsafe {
            cleanup_background_resources();
            if let Some(hwnd) = self.hwnd.take() { 
                let _ = DestroyWindow(hwnd); 
            }
            reset_global_state();
        }
    }
}

impl Drop for NativeScreenshotWindow {
    fn drop(&mut self) { 
        self.cleanup(); 
    }
}

/// 启动原生截屏
#[tauri::command]
pub async fn start_native_screenshot(app: tauri::AppHandle) -> std::result::Result<(), String> {
    // 先隐藏剪贴板窗口
    if let Some(main_window) = app.get_webview_window("main") {
        if main_window.is_visible().unwrap_or(false) {
            crate::window_management::hide_webview_window(main_window);
        }
    }
    // 检查是否已有截屏窗口在运行
    {
        let state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
        if state_guard.is_some() { return Ok(()); }
    }
    let mut screenshot_window = NativeScreenshotWindow::new();
    screenshot_window.start_screenshot().await?;
    Ok(())
}
