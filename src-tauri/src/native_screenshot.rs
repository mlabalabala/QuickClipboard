use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, GenericImageView, ImageFormat};
use screenshots::Screen;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg(windows)]
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::{Dwm::*, Gdi::*},
        System::LibraryLoader::GetModuleHandleW,
        UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, VK_ESCAPE, VK_RETURN},
        UI::WindowsAndMessaging::*,
    },
};

// 屏蔽本文件内的所有调试输出
#[allow(unused_macros)]
macro_rules! println {
    ($($arg:tt)*) => {};
}

// 自定义消息常量
#[cfg(windows)]
const WM_CONFIRM_SCREENSHOT: u32 = WM_USER + 1;

// 拖拽阈值（像素）：按下后移动超过该距离则进入手动拉框，否则视为点击确认智能选区
const DRAG_THRESHOLD: i32 = 4;

// 全局截屏状态
static NATIVE_SCREENSHOT_STATE: Mutex<Option<Arc<NativeScreenshotState>>> = Mutex::new(None);

// 全局退出标志
static SHOULD_EXIT_MESSAGE_LOOP: Mutex<bool> = Mutex::new(false);

/// 调整控制点类型
#[derive(Debug, Clone, Copy, PartialEq)]
enum ResizeHandle {
    TopLeft,
    TopCenter,
    TopRight,
    RightCenter,
    BottomRight,
    BottomCenter,
    BottomLeft,
    LeftCenter,
}

/// 原生截屏状态
#[derive(Clone)]
struct NativeScreenshotState {
    screenshot_image: DynamicImage,
    screen_width: i32,
    screen_height: i32,
    selection_rect: Option<SelectionRect>,
    is_selecting: bool,
    start_point: Option<(i32, i32)>,
    show_confirm_buttons: bool,
    // 点击确认候选（用于智能选区：点击确认 vs 拖动拉框）
    click_confirm_candidate: bool,
    pending_click_point: Option<(i32, i32)>,
    // 鼠标移动判定（避免窗口刚显示就出现智能选区）
    auto_select_enabled: bool,
    last_mouse_point: Option<(i32, i32)>,
    // 双缓冲相关（GDI 已弃用）
    background_dc: Option<HDC>,
    background_bitmap: Option<HBITMAP>,
    last_selection_rect: Option<SelectionRect>, // 用于计算重绘区域
    // 按钮状态
    hover_confirm: bool,        // 确认按钮悬停状态
    hover_cancel: bool,         // 取消按钮悬停状态
    confirm_rect: Option<RECT>, // 确认按钮区域
    cancel_rect: Option<RECT>,  // 取消按钮区域
    // 调整控制点相关
    is_resizing: bool,                        // 是否正在调整选区大小
    resize_handle: Option<ResizeHandle>,      // 当前拖拽的控制点
    hover_handle: Option<ResizeHandle>,       // 当前悬停的控制点
    resize_start_point: Option<(i32, i32)>,   // 调整开始时的鼠标位置
    resize_start_rect: Option<SelectionRect>, // 调整开始时的选区
    // 基础 BGRA 预乘像素缓冲（屏幕截图），仅初始化一次，用于每帧复制
    base_bgra_premul: Option<Vec<u8>>,
    // 半透明黑色 1x1 源，用于 AlphaBlend 叠加蒙版
    overlay_dc: Option<HDC>,
    overlay_bitmap: Option<HBITMAP>,
    // 离屏合成帧缓冲（与窗口同尺寸），用于一次性呈现，避免闪烁
    frame_dc: Option<HDC>,
    frame_bitmap: Option<HBITMAP>,
}

/// 原生截屏窗口管理器
pub struct NativeScreenshotWindow {
    #[cfg(windows)]
    hwnd: Option<HWND>,
}

/// 选区矩形
#[derive(Debug, Clone)]
pub struct SelectionRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
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
            *state = Some(Arc::new(NativeScreenshotState {
                screenshot_image,
                screen_width,
                screen_height,
                selection_rect: None,
                is_selecting: false,
                start_point: None,
                show_confirm_buttons: false,
                click_confirm_candidate: false,
                pending_click_point: None,
                auto_select_enabled: false,
                last_mouse_point: None,
                background_dc: None,
                background_bitmap: None,
                last_selection_rect: None,
                hover_confirm: false,
                hover_cancel: false,
                confirm_rect: None,
                cancel_rect: None,
                is_resizing: false,
                resize_handle: None,
                hover_handle: None,
                resize_start_point: None,
                resize_start_rect: None,
                base_bgra_premul: None,
                overlay_dc: None,
                overlay_bitmap: None,
                frame_dc: None,
                frame_bitmap: None,
            }));
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
            let instance = match GetModuleHandleW(None) { Ok(h) => h, Err(e) => return Err(format!("获取模块句柄失败: {}", e)) };
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
                if !overlay_bits.is_null() { let px = std::slice::from_raw_parts_mut(overlay_bits as *mut u8, 4); px[0] = 0; px[1] = 0; px[2] = 0; px[3] = 120; }

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
            } else { Err("窗口句柄无效".to_string()) }
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
            WM_LBUTTONDOWN => { Self::handle_mouse_down(hwnd, lparam); LRESULT(0) }
            WM_MOUSEMOVE => { Self::handle_mouse_move(hwnd, lparam); LRESULT(0) }
            WM_LBUTTONUP => { Self::handle_mouse_up(hwnd, lparam); LRESULT(0) }
            WM_SETCURSOR => { Self::handle_set_cursor(hwnd); LRESULT(1) }
            // 右键：优先清空选区，无选区时取消截屏
            WM_RBUTTONDOWN => { Self::handle_right_button(hwnd); LRESULT(0) }
            WM_KEYDOWN => {
                if wparam.0 == VK_ESCAPE.0 as usize { let _ = DestroyWindow(hwnd); }
                else if wparam.0 == VK_RETURN.0 as usize { Self::handle_confirm(hwnd); }
                LRESULT(0)
            }
            WM_CONFIRM_SCREENSHOT => { Self::handle_confirm(hwnd); LRESULT(0) }
            WM_DESTROY => { *SHOULD_EXIT_MESSAGE_LOOP.lock().unwrap() = true; LRESULT(0) }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    /// 处理绘制消息 - 背景层 BitBlt + 操作层（仅绘制叠加元素）
    #[cfg(windows)]
    unsafe fn handle_paint(hwnd: HWND) {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);

        let state_arc = { let state = NATIVE_SCREENSHOT_STATE.lock().unwrap(); state.clone() };
        if let Some(state) = state_arc { Self::compose_and_present(hwnd, hdc, &state, ps.rcPaint); }
        else {
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);
            let brush = CreateSolidBrush(COLORREF(0x00000000));
            let _ = FillRect(hdc, &rect, brush);
            let _ = DeleteObject(brush);
        }
        let _ = EndPaint(hwnd, &ps);
    }

    /// 离屏合成：先拷贝背景到 frame_dc，再在 frame_dc 上绘制操作层，最后一次 BitBlt 呈现
    #[cfg(windows)]
    unsafe fn compose_and_present(hwnd: HWND, hdc: HDC, state: &NativeScreenshotState, paint_rect: RECT) {
        if let (Some(bg_dc), Some(frame_dc)) = (state.background_dc, state.frame_dc) {
            let left = paint_rect.left;
            let top = paint_rect.top;
            let width = paint_rect.right - paint_rect.left;
            let height = paint_rect.bottom - paint_rect.top;
            if width <= 0 || height <= 0 { return; }

            let _ = BitBlt(frame_dc, left, top, width, height, bg_dc, left, top, SRCCOPY);
            let _ = IntersectClipRect(frame_dc, left, top, paint_rect.right, paint_rect.bottom);
            Self::draw_operation_layer_to(frame_dc, hwnd, state);
            let _ = SelectClipRgn(frame_dc, HRGN::default());
            let _ = BitBlt(hdc, left, top, width, height, frame_dc, left, top, SRCCOPY);
        }
    }

    /// 在指定 DC 上绘制操作层（选区与按钮等）
    #[cfg(windows)]
    unsafe fn draw_operation_layer_to(target_dc: HDC, hwnd: HWND, state: &NativeScreenshotState) {
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);

        if let Some(sel) = &state.selection_rect {
            if let Some(src_dc) = state.overlay_dc {
                let sx0 = sel.x.max(0);
                let sy0 = sel.y.max(0);
                let sx1 = sel.x + sel.width as i32;
                let sy1 = sel.y + sel.height as i32;

                let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
                if sy0 > 0 { let _ = AlphaBlend(target_dc, 0, 0, client.right, sy0, src_dc, 0, 0, 1, 1, blend); }
                if sy1 < client.bottom { let _ = AlphaBlend(target_dc, 0, sy1, client.right, client.bottom - sy1, src_dc, 0, 0, 1, 1, blend); }
                if sx0 > 0 { let _ = AlphaBlend(target_dc, 0, sy0, sx0, sy1 - sy0, src_dc, 0, 0, 1, 1, blend); }
                if sx1 < client.right { let _ = AlphaBlend(target_dc, sx1, sy0, client.right - sx1, sy1 - sy0, src_dc, 0, 0, 1, 1, blend); }
            }
            Self::draw_selection_rect(target_dc, sel);
            if state.show_confirm_buttons {
                Self::draw_resize_handles(target_dc, sel, state.hover_handle);
                Self::draw_confirm_buttons(target_dc, hwnd, sel);
            }
        } else {
            // 无选区时，整屏半透明遮罩
            if let Some(src_dc) = state.overlay_dc {
                let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
                let _ = AlphaBlend(target_dc, 0, 0, client.right, client.bottom, src_dc, 0, 0, 1, 1, blend);
            }
        }

        // 未固定/未拖拽/未调整时，显示带半透明背景的操作提示；若提示区域处在选区内则隐藏
        if !state.show_confirm_buttons && !state.is_selecting && !state.is_resizing {
            Self::draw_help_panel_and_text(target_dc, hwnd, state);
        }
    }

    /// 绘制带半透明背景的帮助提示；若提示面板位于选区内则不显示
    #[cfg(windows)]
    unsafe fn draw_help_panel_and_text(hdc: HDC, hwnd: HWND, state: &NativeScreenshotState) {
        let help_texts = [
            "拖拽鼠标选择截屏区域",
            "右键 - 取消选区/截屏",
            "ESC - 取消截屏",
            "Enter - 确认截屏",
        ];

        // 计算文本尺寸
        let start_x = 20;
        let start_y = 20;
        let padding = 8;
        let line_spacing = 4;
        let mut max_w = 0i32;
        let mut total_h = 0i32;
        for text in &help_texts {
            let mut text_wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let mut tr = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            let len = text_wide.len() - 1;
            let _ = DrawTextW(hdc, &mut text_wide[..len], &mut tr, DT_CALCRECT);
            let w = tr.right - tr.left;
            let h = tr.bottom - tr.top;
            if w > max_w { max_w = w; }
            total_h += h;
        }
        total_h += line_spacing * (help_texts.len().saturating_sub(1) as i32);

        let panel_rect = RECT {
            left: start_x - padding,
            top: start_y - padding,
            right: start_x + max_w + padding,
            bottom: start_y + total_h + padding,
        };

        // 若面板完全在选区内部，则隐藏
        if let Some(sel) = &state.selection_rect {
            let inside = panel_rect.left >= sel.x
                && panel_rect.top >= sel.y
                && panel_rect.right <= sel.x + sel.width as i32
                && panel_rect.bottom <= sel.y + sel.height as i32;
            if inside { return; }
        }

        // 半透明黑色背景
        if let Some(src_dc) = state.overlay_dc {
            let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
            let _ = AlphaBlend(
                hdc,
                panel_rect.left,
                panel_rect.top,
                panel_rect.right - panel_rect.left,
                panel_rect.bottom - panel_rect.top,
                src_dc,
                0,
                0,
                1,
                1,
                blend,
            );
        }

        // 文本
        Self::draw_help_text(hdc, 0, 0);
    }

    /// 绘制操作提示
    #[cfg(windows)]
    unsafe fn draw_help_text(hdc: HDC, width: i32, height: i32) {
        SetTextColor(hdc, COLORREF(0x00FFFFFF));
        SetBkMode(hdc, TRANSPARENT);
        let help_texts = [
            "拖拽鼠标选择截屏区域",
            "右键 - 取消选区/截屏",
            "ESC - 取消截屏",
            "Enter - 确认截屏",
        ];
        let mut y_offset = 20;
        for text in &help_texts {
            let text_wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let _ = TextOutW(hdc, 20, y_offset, &text_wide[..text_wide.len() - 1]);
            y_offset += 25;
        }
    }

    /// 绘制选区矩形（只绘制边框，不影响背景图像）
    #[cfg(windows)]
    unsafe fn draw_selection_rect(hdc: HDC, selection: &SelectionRect) {
        let rect = RECT { left: selection.x, top: selection.y, right: selection.x + selection.width as i32, bottom: selection.y + selection.height as i32 };
        let pen = CreatePen(PS_SOLID, 3, COLORREF(0x0000FF00));
        let old_pen = SelectObject(hdc, pen);
        MoveToEx(hdc, rect.left, rect.top, None);
        LineTo(hdc, rect.right, rect.top);
        LineTo(hdc, rect.right, rect.bottom);
        LineTo(hdc, rect.left, rect.bottom);
        LineTo(hdc, rect.left, rect.top);
        SelectObject(hdc, old_pen);
        DeleteObject(pen);
        SetTextColor(hdc, COLORREF(0x0000FF00));
        SetBkMode(hdc, TRANSPARENT);
        let info_text = format!("{}x{}", selection.width, selection.height);
        let text_wide: Vec<u16> = info_text.encode_utf16().chain(std::iter::once(0)).collect();
        TextOutW(hdc, rect.right + 5, rect.bottom + 5, &text_wide[..text_wide.len() - 1]);
    }

    /// 绘制确认按钮（现代风格，DPI自适应）
    #[cfg(windows)]
    unsafe fn draw_confirm_buttons(hdc: HDC, hwnd: HWND, selection: &SelectionRect) {
        // DPI 缩放
        let dpi = GetDeviceCaps(hdc, LOGPIXELSX);
        let scale = if dpi > 0 { dpi as f32 / 96.0 } else { 1.0 };
        let s = |v: i32| ((v as f32) * scale).round() as i32;

        // 尺寸参数
        let button_width = s(112);
        let button_height = s(36);
        let button_spacing = s(12);
        let toolbar_padding = s(10);
        let radius_bg = s(12);
        let radius_btn = s(10);
        let border_thin = 1;

        // 屏幕信息
        let screen_width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let screen_height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        let screen_left = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let screen_top = GetSystemMetrics(SM_YVIRTUALSCREEN);

        // 工具栏整体矩形
        let toolbar_width = button_width * 2 + button_spacing + toolbar_padding * 2;
        let toolbar_height = button_height + toolbar_padding * 2;

        let (toolbar_x, toolbar_y) = Self::calculate_toolbar_position(
            selection,
            toolbar_width,
            toolbar_height,
            screen_width,
            screen_height,
            screen_left,
            screen_top,
        );

        // 背景：深色、圆角、细边框
        let toolbar_rect = RECT { left: toolbar_x, top: toolbar_y, right: toolbar_x + toolbar_width, bottom: toolbar_y + toolbar_height };
        Self::draw_toolbar_background(hdc, &toolbar_rect, radius_bg, border_thin);

        // 按钮位置
        let confirm_rect = RECT { left: toolbar_x + toolbar_padding, top: toolbar_y + toolbar_padding, right: toolbar_x + toolbar_padding + button_width, bottom: toolbar_y + toolbar_padding + button_height };
        let cancel_rect = RECT { left: confirm_rect.right + button_spacing, top: confirm_rect.top, right: confirm_rect.right + button_spacing + button_width, bottom: confirm_rect.bottom };

        // 悬停状态
        let is_hover_confirm = if let Some(state) = NATIVE_SCREENSHOT_STATE.lock().unwrap().as_ref() { state.hover_confirm } else { false };
        let is_hover_cancel = if let Some(state) = NATIVE_SCREENSHOT_STATE.lock().unwrap().as_ref() { state.hover_cancel } else { false };

        // 绘制按钮（现代配色）
        Self::draw_button(hdc, &confirm_rect, "✓确认", true, is_hover_confirm, radius_btn, border_thin);
        Self::draw_button(hdc, &cancel_rect, "✗取消", false, is_hover_cancel, radius_btn, border_thin);

        // 更新全局状态中的按钮区域
        if let Some(state) = NATIVE_SCREENSHOT_STATE.lock().unwrap().as_mut() {
            let state_mut = Arc::make_mut(state);
            state_mut.confirm_rect = Some(confirm_rect);
            state_mut.cancel_rect = Some(cancel_rect);
        }
    }

    /// 计算工具栏矩形，供区域失效使用（DPI自适应）
    #[cfg(windows)]
    fn compute_toolbar_rect(selection: &SelectionRect, hwnd: HWND) -> RECT {
        unsafe {
            let hdc = GetDC(hwnd);
            let dpi = if hdc.is_invalid() { 96 } else { let v = GetDeviceCaps(hdc, LOGPIXELSX); let _ = ReleaseDC(hwnd, hdc); if v > 0 { v } else { 96 } };
            let scale = dpi as f32 / 96.0;
            let s = |v: i32| ((v as f32) * scale).round() as i32;

            let button_width = s(112);
            let button_height = s(36);
            let button_spacing = s(12);
            let toolbar_padding = s(10);

            let screen_width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let screen_height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
            let screen_left = GetSystemMetrics(SM_XVIRTUALSCREEN);
            let screen_top = GetSystemMetrics(SM_YVIRTUALSCREEN);

            let toolbar_width = button_width * 2 + button_spacing + toolbar_padding * 2;
            let toolbar_height = button_height + toolbar_padding * 2;

            let (toolbar_x, toolbar_y) = Self::calculate_toolbar_position(
                selection,
                toolbar_width,
                toolbar_height,
                screen_width,
                screen_height,
                screen_left,
                screen_top,
            );
            RECT { left: toolbar_x, top: toolbar_y, right: toolbar_x + toolbar_width, bottom: toolbar_y + toolbar_height }
        }
    }

    /// 绘制工具栏背景（现代风格）
    #[cfg(windows)]
    unsafe fn draw_toolbar_background(hdc: HDC, rect: &RECT, radius: i32, border_thin: i32) {
        // 背景深色
        let background_brush = CreateSolidBrush(COLORREF(0x00222222));
        let old_brush = SelectObject(hdc, background_brush);
        // 细边框（中性灰）
        let border_pen = CreatePen(PS_SOLID, border_thin, COLORREF(0x00484848));
        let old_pen = SelectObject(hdc, border_pen);
        let _ = RoundRect(hdc, rect.left, rect.top, rect.right, rect.bottom, radius, radius);
        let _ = SelectObject(hdc, old_pen);
        let _ = SelectObject(hdc, old_brush);
        let _ = DeleteObject(border_pen);
        let _ = DeleteObject(background_brush);
    }

    /// 绘制按钮（现代风格，主按钮/次按钮）
    #[cfg(windows)]
    unsafe fn draw_button(hdc: HDC, rect: &RECT, text: &str, is_confirm: bool, is_hover: bool, radius: i32, border_thin: i32) {
        // 颜色：主按钮（蓝色），次按钮（中性深灰）
        let (bg_color, hover_bg_color, text_color, border_color) = if is_confirm {
            (COLORREF(0x002A78FF), COLORREF(0x005392FF), COLORREF(0x00FFFFFF), COLORREF(0x003770CC))
        } else {
            (COLORREF(0x00333333), COLORREF(0x00414141), COLORREF(0x00FFFFFF), COLORREF(0x00505050))
        };
        let bg_color = if is_hover { hover_bg_color } else { bg_color };

        let button_brush = CreateSolidBrush(bg_color);
        let border_pen = CreatePen(PS_SOLID, border_thin, border_color);
        let old_brush = SelectObject(hdc, button_brush);
        let old_pen = SelectObject(hdc, border_pen);
        let _ = RoundRect(hdc, rect.left, rect.top, rect.right, rect.bottom, radius, radius);
        let _ = SelectObject(hdc, old_pen);
        let _ = SelectObject(hdc, old_brush);
        let _ = DeleteObject(border_pen);
        let _ = DeleteObject(button_brush);

        // 文本：根据按钮高度自适应字号（约 60% 高度）
        SetTextColor(hdc, text_color);
        SetBkMode(hdc, TRANSPARENT);
        let rect_h = (rect.bottom - rect.top).max(1);
        let font_px = ((rect_h as f32) * 0.6).round() as i32;
        // 负值表示以像素为字符高度
        let font = CreateFontW(-font_px, 0, 0, 0, FW_SEMIBOLD.0 as i32, 0, 0, 0, DEFAULT_CHARSET.0 as u32, OUT_DEFAULT_PRECIS.0 as u32, CLIP_DEFAULT_PRECIS.0 as u32, CLEARTYPE_QUALITY.0 as u32, (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32, w!("Microsoft YaHei"));
        let old_font = SelectObject(hdc, font);
        let mut text_wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let mut text_rect = *rect;
        let text_len = text_wide.len() - 1;
        let _ = DrawTextW(hdc, &mut text_wide[..text_len], &mut text_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        let _ = SelectObject(hdc, old_font);
        let _ = DeleteObject(font);
    }

    /// 获取调整控制点的位置
    fn get_resize_handles(selection: &SelectionRect) -> Vec<(ResizeHandle, RECT)> {
        let handle_size = 8;
        let half_size = handle_size / 2;
        let left = selection.x;
        let top = selection.y;
        let right = selection.x + selection.width as i32;
        let bottom = selection.y + selection.height as i32;
        let center_x = left + (selection.width as i32) / 2;
        let center_y = top + (selection.height as i32) / 2;
        vec![
            (ResizeHandle::TopLeft, RECT { left: left - half_size, top: top - half_size, right: left + half_size, bottom: top + half_size }),
            (ResizeHandle::TopCenter, RECT { left: center_x - half_size, top: top - half_size, right: center_x + half_size, bottom: top + half_size }),
            (ResizeHandle::TopRight, RECT { left: right - half_size, top: top - half_size, right: right + half_size, bottom: top + half_size }),
            (ResizeHandle::RightCenter, RECT { left: right - half_size, top: center_y - half_size, right: right + half_size, bottom: center_y + half_size }),
            (ResizeHandle::BottomRight, RECT { left: right - half_size, top: bottom - half_size, right: right + half_size, bottom: bottom + half_size }),
            (ResizeHandle::BottomCenter, RECT { left: center_x - half_size, top: bottom - half_size, right: center_x + half_size, bottom: bottom + half_size }),
            (ResizeHandle::BottomLeft, RECT { left: left - half_size, top: bottom - half_size, right: left + half_size, bottom: bottom + half_size }),
            (ResizeHandle::LeftCenter, RECT { left: left - half_size, top: center_y - half_size, right: left + half_size, bottom: center_y + half_size }),
        ]
    }

    /// 检测鼠标是否在调整控制点上
    fn hit_test_resize_handles(selection: &SelectionRect, x: i32, y: i32) -> Option<ResizeHandle> {
        let handles = Self::get_resize_handles(selection);
        for (handle, rect) in handles { if x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom { return Some(handle); } }
        None
    }

    /// 绘制调整控制点
    #[cfg(windows)]
    unsafe fn draw_resize_handles(
        hdc: HDC,
        selection: &SelectionRect,
        hover_handle: Option<ResizeHandle>,
    ) {
        let handles = Self::get_resize_handles(selection);
        for (handle, rect) in handles {
            let is_hover = hover_handle == Some(handle);
            let fill_color = if is_hover { COLORREF(0x0000FFFF) } else { COLORREF(0x00FFFFFF) };
            let border_color = COLORREF(0x0000FF00);
            let brush = CreateSolidBrush(fill_color);
            let pen = CreatePen(PS_SOLID, 2, border_color);
            let old_brush = SelectObject(hdc, brush);
            let old_pen = SelectObject(hdc, pen);
            Rectangle(hdc, rect.left, rect.top, rect.right, rect.bottom);
            SelectObject(hdc, old_pen);
            SelectObject(hdc, old_brush);
            DeleteObject(pen);
            DeleteObject(brush);
        }
    }

    /// 根据控制点类型设置鼠标样式
    #[cfg(windows)]
    unsafe fn set_cursor_for_handle(handle: ResizeHandle) {
        let cursor_id = match handle {
            ResizeHandle::TopLeft | ResizeHandle::BottomRight => IDC_SIZENWSE,
            ResizeHandle::TopRight | ResizeHandle::BottomLeft => IDC_SIZENESW,
            ResizeHandle::TopCenter | ResizeHandle::BottomCenter => IDC_SIZENS,
            ResizeHandle::LeftCenter | ResizeHandle::RightCenter => IDC_SIZEWE,
        };
        if let Ok(cursor) = LoadCursorW(None, cursor_id) { SetCursor(cursor); }
    }

    /// 根据控制点拖拽更新选区
    fn update_selection_with_handle(
        handle: ResizeHandle,
        start_rect: &SelectionRect,
        start_mouse: (i32, i32),
        current_mouse: (i32, i32),
    ) -> SelectionRect {
        let dx = current_mouse.0 - start_mouse.0;
        let dy = current_mouse.1 - start_mouse.1;
        let mut new_x = start_rect.x;
        let mut new_y = start_rect.y;
        let mut new_width = start_rect.width as i32;
        let mut new_height = start_rect.height as i32;
        match handle {
            ResizeHandle::TopLeft => { new_x += dx; new_y += dy; new_width -= dx; new_height -= dy; }
            ResizeHandle::TopCenter => { new_y += dy; new_height -= dy; }
            ResizeHandle::TopRight => { new_y += dy; new_width += dx; new_height -= dy; }
            ResizeHandle::RightCenter => { new_width += dx; }
            ResizeHandle::BottomRight => { new_width += dx; new_height += dy; }
            ResizeHandle::BottomCenter => { new_height += dy; }
            ResizeHandle::BottomLeft => { new_x += dx; new_width -= dx; new_height += dy; }
            ResizeHandle::LeftCenter => { new_x += dx; new_width -= dx; }
        }
        const MIN_SIZE: i32 = 10;
        if new_width < MIN_SIZE { if handle == ResizeHandle::TopLeft || handle == ResizeHandle::BottomLeft || handle == ResizeHandle::LeftCenter { new_x = new_x + new_width - MIN_SIZE; } new_width = MIN_SIZE; }
        if new_height < MIN_SIZE { if handle == ResizeHandle::TopLeft || handle == ResizeHandle::TopCenter || handle == ResizeHandle::TopRight { new_y = new_y + new_height - MIN_SIZE; } new_height = MIN_SIZE; }
        SelectionRect { x: new_x, y: new_y, width: new_width as u32, height: new_height as u32 }
    }

    /// 计算工具栏的最佳位置，避免超出屏幕
    fn calculate_toolbar_position(
        selection: &SelectionRect,
        toolbar_width: i32,
        toolbar_height: i32,
        screen_width: i32,
        screen_height: i32,
        screen_left: i32,
        screen_top: i32,
    ) -> (i32, i32) {
        let margin = 10;
        let virtual_right = screen_left + screen_width;
        let virtual_bottom = screen_top + screen_height;
        let preferred_x = selection.x + (selection.width as i32 - toolbar_width) / 2;
        let preferred_y = selection.y + selection.height as i32 + margin;
        if preferred_y + toolbar_height <= virtual_bottom - margin {
            let final_x = if preferred_x < screen_left + margin { screen_left + margin } else if preferred_x + toolbar_width > virtual_right - margin { virtual_right - toolbar_width - margin } else { preferred_x };
            return (final_x, preferred_y);
        }
        let top_y = selection.y - toolbar_height - margin;
        if top_y >= screen_top + margin {
            let final_x = if preferred_x < screen_left + margin { screen_left + margin } else if preferred_x + toolbar_width > virtual_right - margin { virtual_right - toolbar_width - margin } else { preferred_x };
            return (final_x, top_y);
        }
        let right_x = selection.x + selection.width as i32 + margin;
        if right_x + toolbar_width <= virtual_right - margin {
            let center_y = selection.y + (selection.height as i32 - toolbar_height) / 2;
            let final_y = if center_y < screen_top + margin { screen_top + margin } else if center_y + toolbar_height > virtual_bottom - margin { virtual_bottom - toolbar_height - margin } else { center_y };
            return (right_x, final_y);
        }
        let left_x = selection.x - toolbar_width - margin;
        if left_x >= screen_left + margin {
            let center_y = selection.y + (selection.height as i32 - toolbar_height) / 2;
            let final_y = if center_y < screen_top + margin { screen_top + margin } else if center_y + toolbar_height > virtual_bottom - margin { virtual_bottom - toolbar_height - margin } else { center_y };
            return (left_x, final_y);
        }
        (virtual_right - toolbar_width - margin, virtual_bottom - toolbar_height - margin)
    }

    /// 智能选区：根据鼠标位置自动捕捉到下方窗口/子窗口矩形
    #[cfg(windows)]
    unsafe fn get_auto_selection_at_point(hwnd_overlay: HWND, x: i32, y: i32) -> Option<SelectionRect> {
        // 将客户端坐标转换为屏幕坐标
        let mut sp = POINT { x, y };
        let _ = ClientToScreen(hwnd_overlay, &mut sp);
        let screen_left = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let screen_top = GetSystemMetrics(SM_YVIRTUALSCREEN);

        // 直接基于Z序扫描叠加层之下的窗口，避免命中透明样式切换导致光标闪烁
        let mut target = HWND(0);
        let mut iter = GetWindow(hwnd_overlay, GW_HWNDPREV);
        while iter.0 != 0 {
            if IsWindowVisible(iter).as_bool() {
                let mut r = RECT::default();
                if GetWindowRect(iter, &mut r).is_ok() && PtInRect(&r, sp).as_bool() {
                    target = iter;
                    break;
                }
            }
            iter = GetWindow(iter, GW_HWNDPREV);
        }
        if target.0 == 0 {
            // 回退：全局枚举顶层窗口，取第一个包含点的窗口
            let mut data = EnumFindData { point: sp, overlay: hwnd_overlay, found: HWND(0) };
            let _ = EnumWindows(Some(enum_windows_find_under_point), LPARAM(&mut data as *mut _ as isize));
            if data.found.0 == 0 { return None; }
            target = data.found;
        }

        // 跳过桌面类窗口，避免全屏
        let mut class_buf = [0u16; 256];
        let len = GetClassNameW(target, &mut class_buf);
        if len > 0 {
            let cls = String::from_utf16_lossy(&class_buf[..len as usize]);
            if cls == "Progman" || cls == "WorkerW" {
                return None;
            }
        }

        // 递归查找更深的子窗口
        let flags = CWP_SKIPDISABLED | CWP_SKIPINVISIBLE | CWP_SKIPTRANSPARENT;
        let mut deepest = target;
        loop {
            let mut pt_client = POINT { x: sp.x, y: sp.y };
            let _ = ScreenToClient(deepest, &mut pt_client);
            let child = ChildWindowFromPointEx(deepest, pt_client, flags);
            if child.0 != 0 && child != deepest {
                let mut cr = RECT::default();
                if GetWindowRect(child, &mut cr).is_ok() && PtInRect(&cr, sp).as_bool() {
                    deepest = child;
                    continue;
                }
            }
            break;
        }

        let mut wr = RECT::default();
        if !GetWindowRect(deepest, &mut wr).is_ok() { return None; }

        // 顶层窗口使用 DWM 扩展边界，去除阴影带来的左右/下间隙
        let root = GetAncestor(deepest, GA_ROOT);
        if root == deepest {
            let mut efr = RECT::default();
            if DwmGetWindowAttribute(
                deepest,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut efr as *mut _ as *mut core::ffi::c_void,
                std::mem::size_of::<RECT>() as u32,
            ).is_ok() {
                wr = efr;
            }
        }

        // 转换为叠加窗口坐标系（使用 MapWindowPoints 从屏幕坐标映射到叠加层客户端坐标）
        let mut pts = [POINT { x: wr.left, y: wr.top }, POINT { x: wr.right, y: wr.bottom }];
        let _ = MapWindowPoints(HWND(0), hwnd_overlay, &mut pts);
        let x0 = pts[0].x;
        let y0 = pts[0].y;
        let w = (pts[1].x - pts[0].x).max(0);
        let h = (pts[1].y - pts[0].y).max(0);
        if w <= 0 || h <= 0 { return None; }

        // 裁剪到叠加窗口的可视区域
        let mut client = RECT::default();
        let _ = GetClientRect(hwnd_overlay, &mut client);
        let rx0 = x0.clamp(0, client.right);
        let ry0 = y0.clamp(0, client.bottom);
        let rx1 = (x0 + w).clamp(0, client.right);
        let ry1 = (y0 + h).clamp(0, client.bottom);
        let fw = (rx1 - rx0).max(0);
        let fh = (ry1 - ry0).max(0);
        // 避免全屏覆盖（等于叠加层尺寸时返回None）
        if rx0 == 0 && ry0 == 0 && fw == client.right && fh == client.bottom { return None; }
        if fw <= 0 || fh <= 0 { return None; }

        Some(SelectionRect { x: rx0, y: ry0, width: fw as u32, height: fh as u32 })
    }

    #[cfg(windows)]
    fn run_message_loop(&self) -> std::result::Result<(), String> {
        unsafe {
            let mut msg = MSG::default();
            loop {
                if *SHOULD_EXIT_MESSAGE_LOOP.lock().unwrap() { break; }
                let result = if let Some(hwnd) = self.hwnd { GetMessageW(&mut msg, hwnd, 0, 0) } else { GetMessageW(&mut msg, None, 0, 0) };
                if result.0 == 0 { break; }
                else if result.0 == -1 { return Err("消息循环错误".to_string()); }
                else { let _ = TranslateMessage(&msg); DispatchMessageW(&msg); }
            }
        }
        Ok(())
    }

    pub fn cleanup(&mut self) {
        #[cfg(windows)]
        unsafe {
            cleanup_background_resources();
            if let Some(hwnd) = self.hwnd.take() { let _ = DestroyWindow(hwnd); }
            reset_global_state();
        }
    }

    #[cfg(windows)]
    unsafe fn handle_mouse_down(hwnd: HWND, lparam: LPARAM) {
        let x = (lparam.0 & 0xFFFF) as i16 as i32;
        let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
        let mut button_clicked = false;
        {
            let state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
            if let Some(ref state_arc) = *state_guard {
                if state_arc.show_confirm_buttons {
                    if let Some(ref confirm_rect) = state_arc.confirm_rect {
                        if x >= confirm_rect.left && x <= confirm_rect.right && y >= confirm_rect.top && y <= confirm_rect.bottom {
                            let _ = PostMessageW(hwnd, WM_CONFIRM_SCREENSHOT, WPARAM(0), LPARAM(0));
                            button_clicked = true;
                        }
                    }
                    if !button_clicked {
                        if let Some(ref cancel_rect) = state_arc.cancel_rect {
                            if x >= cancel_rect.left && x <= cancel_rect.right && y >= cancel_rect.top && y <= cancel_rect.bottom {
                                let _ = DestroyWindow(hwnd);
                                button_clicked = true;
                            }
                        }
                    }
                }
            }
        }
        if !button_clicked {
            let mut resize_handle_clicked = None;
            {
                let state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
                if let Some(ref state_arc) = *state_guard {
                    if let Some(ref selection) = state_arc.selection_rect {
                        if state_arc.show_confirm_buttons {
                            resize_handle_clicked = Self::hit_test_resize_handles(selection, x, y);
                        }
                    }
                }
            }
            if let Some(handle) = resize_handle_clicked {
                let mut state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
                if let Some(ref mut state_arc) = *state_guard {
                    if let Some(state) = Arc::get_mut(state_arc) {
                        state.is_resizing = true;
                        state.resize_handle = Some(handle);
                        state.resize_start_point = Some((x, y));
                        state.resize_start_rect = state.selection_rect.clone();
                    }
                }
                let _ = SetCapture(hwnd);
            } else {
                // 智能选区无缝切换：点击=确认，拖动=拉框
                let mut started_drag = false;
                {
                    let mut state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
                    if let Some(ref mut state_arc) = *state_guard {
                        if let Some(state) = Arc::get_mut(state_arc) {
                            if let Some(sel) = &state.selection_rect {
                                if x >= sel.x && x <= sel.x + sel.width as i32 && y >= sel.y && y <= sel.y + sel.height as i32 {
                                    // 点击在选区内：标记为确认候选，等待鼠标移动判断意图
                                    state.click_confirm_candidate = true;
                                    state.pending_click_point = Some((x, y));
                                } else {
                                    // 点击在选区外：开始新的拖拽选区
                                    state.is_selecting = true;
                                    state.start_point = Some((x, y));
                                    state.selection_rect = Some(SelectionRect { x, y, width: 0, height: 0 });
                                    state.show_confirm_buttons = false;
                                    started_drag = true;
                                }
                            } else {
                                // 没有选区：开始新的拖拽选区
                                state.is_selecting = true;
                                state.start_point = Some((x, y));
                                state.selection_rect = Some(SelectionRect { x, y, width: 0, height: 0 });
                                state.show_confirm_buttons = false;
                                started_drag = true;
                            }
                        }
                    }
                }
                let _ = SetCapture(hwnd);
                if started_drag { let _ = InvalidateRect(hwnd, None, FALSE); }
            }
            let _ = InvalidateRect(hwnd, None, FALSE);
        }
    }

    #[cfg(windows)]
    unsafe fn handle_mouse_move(hwnd: HWND, lparam: LPARAM) {
        let x = (lparam.0 & 0xFFFF) as i16 as i32;
        let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as i32;
        let mut should_redraw = false;
        let mut dirty_rect: Option<RECT> = None;
        let mut force_full_invalidate = false;
        // 鼠标移动记录：第一次真实移动后才启用智能选区
        {
            let mut state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
            if let Some(ref mut state_arc) = *state_guard {
                if let Some(state) = Arc::get_mut(state_arc) {
                    match state.last_mouse_point {
                        None => { state.last_mouse_point = Some((x, y)); }
                        Some((px, py)) => {
                            if (x != px || y != py) && !state.auto_select_enabled {
                                state.auto_select_enabled = true;
                            }
                            state.last_mouse_point = Some((x, y));
                        }
                    }
                    
                    // 智能判断：如果点击确认候选且鼠标移动超过阈值，转换为拖拽模式
                    if state.click_confirm_candidate && state.pending_click_point.is_some() {
                        if let Some((click_x, click_y)) = state.pending_click_point {
                            let move_distance = ((x - click_x).pow(2) + (y - click_y).pow(2)) as f64;
                            if move_distance.sqrt() > 5.0 { // 5像素的移动阈值
                                // 转换为拖拽模式
                                state.is_selecting = true;
                                state.start_point = Some((click_x, click_y));
                                state.selection_rect = Some(SelectionRect { x: click_x, y: click_y, width: 0, height: 0 });
                                state.show_confirm_buttons = false;
                                state.click_confirm_candidate = false;
                                state.pending_click_point = None;
                                should_redraw = true;
                                force_full_invalidate = true;
                            }
                        }
                    }
                }
            }
        }
        {
            let mut state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
            if let Some(ref mut state_arc) = *state_guard {
                if let Some(state) = Arc::get_mut(state_arc) {
                    if state.is_selecting {
                        if let Some((start_x, start_y)) = state.start_point {
                            let width = (x - start_x).abs() as u32;
                            let height = (y - start_y).abs() as u32;
                            let rect_x = start_x.min(x);
                            let rect_y = start_y.min(y);
                            let new_rect = SelectionRect { x: rect_x, y: rect_y, width, height };
                            let prev_rect_opt = state.selection_rect.clone();
                            if let Some(ref current_rect) = prev_rect_opt {
                                if current_rect.x != new_rect.x || current_rect.y != new_rect.y || current_rect.width != new_rect.width || current_rect.height != new_rect.height {
                                    should_redraw = true;
                                    force_full_invalidate = true;
                                    let mut client = RECT::default();
                                    let _ = GetClientRect(hwnd, &mut client);
                                    let margin: i32 = 32;
                                    let prev = RECT { left: current_rect.x, top: current_rect.y, right: current_rect.x + current_rect.width as i32, bottom: current_rect.y + current_rect.height as i32 };
                                    let next = RECT { left: new_rect.x, top: new_rect.y, right: new_rect.x + new_rect.width as i32, bottom: new_rect.y + new_rect.height as i32 };
                                    let mut left = prev.left.min(next.left) - margin;
                                    let mut top = prev.top.min(next.top) - margin;
                                    let mut right = prev.right.max(next.right) + margin;
                                    let mut bottom = prev.bottom.max(next.bottom) + margin;
                                    if state.show_confirm_buttons {
                                        let prev_toolbar = Self::compute_toolbar_rect(current_rect, hwnd);
                                        let next_toolbar = Self::compute_toolbar_rect(&new_rect, hwnd);
                                        left = left.min(prev_toolbar.left).min(next_toolbar.left) - margin;
                                        top = top.min(prev_toolbar.top).min(next_toolbar.top) - margin;
                                        right = right.max(prev_toolbar.right).max(next_toolbar.right) + margin;
                                        bottom = bottom.max(prev_toolbar.bottom).max(next_toolbar.bottom) + margin;
                                    }
                                    if left < client.left { left = client.left; }
                                    if top < client.top { top = client.top; }
                                    if right > client.right { right = client.right; }
                                    if bottom > client.bottom { bottom = client.bottom; }
                                    if right > left && bottom > top { dirty_rect = Some(RECT { left, top, right, bottom }); }
                                }
                            } else {
                                should_redraw = true;
                                force_full_invalidate = true;
                            }
                            state.selection_rect = Some(new_rect.clone());
                            state.last_selection_rect = Some(new_rect);
                        }
                    } else if state.is_resizing {
                        if let (Some(handle), Some(start_mouse), Some(ref start_rect)) = (state.resize_handle, state.resize_start_point, &state.resize_start_rect) {
                            let new_rect = Self::update_selection_with_handle(handle, start_rect, start_mouse, (x, y));
                            let prev_rect_opt = state.selection_rect.clone();
                            if let Some(ref current_rect) = prev_rect_opt {
                                if current_rect.x != new_rect.x || current_rect.y != new_rect.y || current_rect.width != new_rect.width || current_rect.height != new_rect.height {
                                    should_redraw = true;
                                    force_full_invalidate = true;
                                    let mut client = RECT::default();
                                    let _ = GetClientRect(hwnd, &mut client);
                                    let margin: i32 = 32;
                                    let prev = RECT { left: current_rect.x, top: current_rect.y, right: current_rect.x + current_rect.width as i32, bottom: current_rect.y + current_rect.height as i32 };
                                    let next = RECT { left: new_rect.x, top: new_rect.y, right: new_rect.x + new_rect.width as i32, bottom: new_rect.y + new_rect.height as i32 };
                                    let mut left = prev.left.min(next.left) - margin;
                                    let mut top = prev.top.min(next.top) - margin;
                                    let mut right = prev.right.max(next.right) + margin;
                                    let mut bottom = prev.bottom.max(next.bottom) + margin;
                                    if state.show_confirm_buttons {
                                        let prev_toolbar = Self::compute_toolbar_rect(current_rect, hwnd);
                                        let next_toolbar = Self::compute_toolbar_rect(&new_rect, hwnd);
                                        left = left.min(prev_toolbar.left).min(next_toolbar.left) - margin;
                                        top = top.min(prev_toolbar.top).min(next_toolbar.top) - margin;
                                        right = right.max(prev_toolbar.right).max(next_toolbar.right) + margin;
                                        bottom = bottom.max(prev_toolbar.bottom).max(next_toolbar.bottom) + margin;
                                    }
                                    if left < client.left { left = client.left; }
                                    if top < client.top { top = client.top; }
                                    if right > client.right { right = client.right; }
                                    if bottom > client.bottom { bottom = client.bottom; }
                                    if right > left && bottom > top { dirty_rect = Some(RECT { left, top, right, bottom }); }
                                }
                            } else {
                                should_redraw = true;
                                force_full_invalidate = true;
                            }
                            state.selection_rect = Some(new_rect.clone());
                            state.last_selection_rect = Some(new_rect);
                        }
                    }
                }
            }
        }

        // 智能选区：未在拖拽/调整且未固定选区且已检测到实际鼠标移动时，根据鼠标自动高亮元素
        {
            let mut state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
            if let Some(ref mut state_arc) = *state_guard {
                if let Some(state) = Arc::get_mut(state_arc) {
                    if !state.is_selecting && !state.is_resizing && !state.show_confirm_buttons && state.auto_select_enabled {
                        if let Some(auto_rect) = Self::get_auto_selection_at_point(hwnd, x, y) {
                            let changed = match &state.selection_rect {
                                Some(cur) => cur.x != auto_rect.x || cur.y != auto_rect.y || cur.width != auto_rect.width || cur.height != auto_rect.height,
                                None => true,
                            };
                            if changed {
                                state.selection_rect = Some(auto_rect);
                                should_redraw = true;
                            }
                        }
                    }
                }
            }
        }

        let mut hover_changed = false;
        {
            let mut state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
            if let Some(ref mut state_arc) = *state_guard {
                if let Some(state) = Arc::get_mut(state_arc) {
                    if state.show_confirm_buttons && !state.is_selecting && !state.is_resizing {
                        let mut new_hover_confirm = false;
                        let mut new_hover_cancel = false;
                        let mut new_hover_handle = None;
                        if let Some(ref confirm_rect) = state.confirm_rect { if x >= confirm_rect.left && x <= confirm_rect.right && y >= confirm_rect.top && y <= confirm_rect.bottom { new_hover_confirm = true; } }
                        if let Some(ref cancel_rect) = state.cancel_rect { if x >= cancel_rect.left && x <= cancel_rect.right && y >= cancel_rect.top && y <= cancel_rect.bottom { new_hover_cancel = true; } }
                        if !new_hover_confirm && !new_hover_cancel { if let Some(ref selection) = state.selection_rect { new_hover_handle = Self::hit_test_resize_handles(selection, x, y); } }
                        if state.hover_confirm != new_hover_confirm || state.hover_cancel != new_hover_cancel || state.hover_handle != new_hover_handle { state.hover_confirm = new_hover_confirm; state.hover_cancel = new_hover_cancel; state.hover_handle = new_hover_handle; hover_changed = true; }
                    }
                }
            }
        }
        if force_full_invalidate { let _ = InvalidateRect(hwnd, None, FALSE); }
        else if let Some(rect) = dirty_rect { let _ = InvalidateRect(hwnd, Some(&rect), FALSE); }
        else if should_redraw || hover_changed { let _ = InvalidateRect(hwnd, None, FALSE); }
    }

    #[cfg(windows)]
    unsafe fn handle_mouse_up(hwnd: HWND, _lparam: LPARAM) {
        let mut state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
        if let Some(ref mut state_arc) = *state_guard {
            if let Some(state) = Arc::get_mut(state_arc) {
                if state.is_selecting {
                    state.is_selecting = false;
                    if let Some(selection) = &state.selection_rect { if selection.width > 10 && selection.height > 10 { state.show_confirm_buttons = true; let _ = InvalidateRect(hwnd, None, FALSE); } }
                } else if state.is_resizing {
                    state.is_resizing = false; state.resize_handle = None; state.resize_start_point = None; state.resize_start_rect = None;
                    let _ = InvalidateRect(hwnd, None, FALSE);
                } else if state.click_confirm_candidate {
                    state.click_confirm_candidate = false;
                    state.pending_click_point = None;
                    if let Some(selection) = &state.selection_rect {
                        if selection.width > 10 && selection.height > 10 && !state.show_confirm_buttons {
                            state.show_confirm_buttons = true;
                            let _ = InvalidateRect(hwnd, None, FALSE);
                        }
                    }
                }
            }
        }
        let _ = ReleaseCapture();
    }

    #[cfg(windows)]
    unsafe fn handle_confirm(hwnd: HWND) {
        let selection_data = {
            let state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
            if let Some(ref state_arc) = *state_guard {
                if let Some(ref selection) = state_arc.selection_rect {
                    if selection.width > 10 && selection.height > 10 { Some((state_arc.screenshot_image.clone(), selection.clone())) } else { None }
                } else { None }
            } else { None }
        };
        let _ = DestroyWindow(hwnd);
        if let Some((image, selection)) = selection_data {
            std::thread::sleep(std::time::Duration::from_millis(100));
            let _ = crop_and_save_to_clipboard(&image, &selection);
        }
    }

    /// 统一在 WM_SETCURSOR 中设置光标，避免闪烁
    #[cfg(windows)]
    unsafe fn handle_set_cursor(hwnd: HWND) {
        let mut pt_screen = POINT::default();
        if !GetCursorPos(&mut pt_screen).is_ok() { return; }
        let mut pt_client = pt_screen;
        let _ = ScreenToClient(hwnd, &mut pt_client);

        let state_arc = { let s = NATIVE_SCREENSHOT_STATE.lock().unwrap(); s.clone() };
        if let Some(state) = state_arc {
            if state.is_resizing {
                if let Some(handle) = state.resize_handle { Self::set_cursor_for_handle(handle); return; }
            }
            if state.show_confirm_buttons && !state.is_selecting && !state.is_resizing {
                if let Some(r) = state.confirm_rect {
                    if pt_client.x >= r.left && pt_client.x <= r.right && pt_client.y >= r.top && pt_client.y <= r.bottom {
                        if let Ok(cur) = LoadCursorW(None, IDC_HAND) { SetCursor(cur); return; }
                    }
                }
                if let Some(r) = state.cancel_rect {
                    if pt_client.x >= r.left && pt_client.x <= r.right && pt_client.y >= r.top && pt_client.y <= r.bottom {
                        if let Ok(cur) = LoadCursorW(None, IDC_HAND) { SetCursor(cur); return; }
                    }
                }
                if let Some(ref sel) = state.selection_rect {
                    if let Some(handle) = Self::hit_test_resize_handles(sel, pt_client.x, pt_client.y) {
                        Self::set_cursor_for_handle(handle);
                        return;
                    }
                }
            }
        }
        if let Ok(cur) = LoadCursorW(None, IDC_CROSS) { SetCursor(cur); }
    }

    /// 右键处理：先清空选区/状态，再决定是否关闭
    #[cfg(windows)]
    unsafe fn handle_right_button(hwnd: HWND) {
        let mut should_close = false;
        {
            let mut state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
            if let Some(ref mut state_arc) = *state_guard {
                if let Some(state) = Arc::get_mut(state_arc) {
                    let has_fixed_selection = state.show_confirm_buttons || state.is_selecting || state.is_resizing;
                    let has_ephemeral_selection = state.selection_rect.is_some() && !has_fixed_selection;

                    // 重置所有与选区相关的状态
                    state.is_selecting = false;
                    state.is_resizing = false;
                    state.selection_rect = None;
                    state.start_point = None;
                    state.show_confirm_buttons = false;
                    state.hover_confirm = false;
                    state.hover_cancel = false;
                    state.hover_handle = None;
                    state.confirm_rect = None;
                    state.cancel_rect = None;
                    state.resize_handle = None;
                    state.resize_start_point = None;
                    state.resize_start_rect = None;
                    state.click_confirm_candidate = false;
                    state.pending_click_point = None;

                    // 未确认的智能选区或无选区时，右键直接退出；有固定/正在操作选区时仅清空
                    if !has_fixed_selection {
                        // 包含：has_ephemeral_selection 或无任何选区
                        should_close = true;
                    }
                }
            } else {
                // 无全局状态时直接关闭
                should_close = true;
            }
        }
        if should_close { let _ = DestroyWindow(hwnd); }
        else { let _ = InvalidateRect(hwnd, None, FALSE); }
    }
}

impl Drop for NativeScreenshotWindow {
    fn drop(&mut self) { self.cleanup(); }
}

// 资源清理（包含背景、叠加源与离屏帧缓冲）
#[cfg(windows)]
unsafe fn cleanup_background_resources() {
    let mut state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
    if let Some(ref mut state_arc) = *state_guard {
        if let Some(state) = Arc::get_mut(state_arc) {
            if let Some(dc) = state.background_dc.take() { let _ = DeleteDC(dc); }
            if let Some(bitmap) = state.background_bitmap.take() { let _ = DeleteObject(bitmap); }
            if let Some(ovdc) = state.overlay_dc.take() { let _ = DeleteDC(ovdc); }
            if let Some(ovbmp) = state.overlay_bitmap.take() { let _ = DeleteObject(ovbmp); }
            if let Some(fdc) = state.frame_dc.take() { let _ = DeleteDC(fdc); }
            if let Some(fbmp) = state.frame_bitmap.take() { let _ = DeleteObject(fbmp); }
            state.base_bgra_premul = None;
        }
    }
}

#[cfg(windows)]
unsafe fn reset_global_state() {
    let mut state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
    *state_guard = None;
}

// 裁剪并保存到剪贴板（与旧版一致）
#[cfg(windows)]
fn crop_and_save_to_clipboard(
    image: &DynamicImage,
    selection: &SelectionRect,
) -> std::result::Result<(), String> {
    if selection.width == 0 || selection.height == 0 { return Err("选区尺寸无效".to_string()); }
    let (img_width, img_height) = image.dimensions();
    if selection.x < 0 || selection.y < 0 || selection.x as u32 + selection.width > img_width || selection.y as u32 + selection.height > img_height { return Err("选区超出图像范围".to_string()); }
    let cropped = image.crop_imm(selection.x as u32, selection.y as u32, selection.width, selection.height);
    let quality = crate::settings::get_global_settings().screenshot_quality;
    let data_url = image_to_data_url_with_quality(&cropped, quality)?;
    use crate::clipboard_content::set_clipboard_content_no_history;
    set_clipboard_content_no_history(data_url).map_err(|e| format!("保存截屏到剪贴板失败: {}", e))?;
    Ok(())
}

/// 根据质量设置将图像转换为data URL
fn image_to_data_url_with_quality(
    image: &DynamicImage,
    quality: u8,
) -> std::result::Result<String, String> {
    let mut buffer = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buffer);
    if quality == 100 {
        image.write_to(&mut cursor, ImageFormat::Png).map_err(|e| format!("PNG编码失败: {}", e))?;
        let base64_data = general_purpose::STANDARD.encode(&buffer);
        Ok(format!("data:image/png;base64,{}", base64_data))
    } else {
        let jpeg_encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality);
        image.write_with_encoder(jpeg_encoder).map_err(|e| format!("JPEG编码失败: {}", e))?;
        let base64_data = general_purpose::STANDARD.encode(&buffer);
        Ok(format!("data:image/jpeg;base64,{}", base64_data))
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

#[cfg(windows)]
#[repr(C)]
struct EnumFindData {
    point: POINT,
    overlay: HWND,
    found: HWND,
}

#[cfg(windows)]
unsafe extern "system" fn enum_windows_find_under_point(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let data = &mut *(lparam.0 as *mut EnumFindData);
    if hwnd == data.overlay { return BOOL(1); }
    if !IsWindowVisible(hwnd).as_bool() { return BOOL(1); }
    let mut r = RECT::default();
    if GetWindowRect(hwnd, &mut r).is_ok() && PtInRect(&r, data.point).as_bool() {
        let mut class_buf = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut class_buf);
        if len > 0 {
            let cls = String::from_utf16_lossy(&class_buf[..len as usize]);
            if cls == "Progman" || cls == "WorkerW" { return BOOL(1); }
        }
        data.found = hwnd;
        return BOOL(0); // stop enumeration
    }
    BOOL(1)
}
