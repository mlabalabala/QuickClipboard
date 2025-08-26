use image::DynamicImage;
use std::sync::{Arc, Mutex};

#[cfg(windows)]
use windows::Win32::{
    Foundation::{RECT},
    Graphics::Gdi::{HDC, HBITMAP},
};

// 拖拽阈值（像素）：按下后移动超过该距离则进入手动拉框，否则视为点击确认智能选区
pub const DRAG_THRESHOLD: i32 = 4;

// 全局截屏状态
pub static NATIVE_SCREENSHOT_STATE: Mutex<Option<Arc<NativeScreenshotState>>> = Mutex::new(None);

// 全局退出标志
pub static SHOULD_EXIT_MESSAGE_LOOP: Mutex<bool> = Mutex::new(false);

/// 调整控制点类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeHandle {
    TopLeft,
    TopCenter,
    TopRight,
    RightCenter,
    BottomRight,
    BottomCenter,
    BottomLeft,
    LeftCenter,
}

/// 选区矩形
#[derive(Debug, Clone)]
pub struct SelectionRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 原生截屏状态
#[derive(Clone)]
pub struct NativeScreenshotState {
    pub screenshot_image: DynamicImage,
    pub screen_width: i32,
    pub screen_height: i32,
    pub selection_rect: Option<SelectionRect>,
    pub is_selecting: bool,
    pub start_point: Option<(i32, i32)>,
    pub show_confirm_buttons: bool,
    // 点击确认候选（用于智能选区：点击确认 vs 拖动拉框）
    pub click_confirm_candidate: bool,
    pub pending_click_point: Option<(i32, i32)>,
    // 鼠标移动判定（避免窗口刚显示就出现智能选区）
    pub auto_select_enabled: bool,
    pub last_mouse_point: Option<(i32, i32)>,
    // 双缓冲相关（GDI 已弃用）
    #[cfg(windows)]
    pub background_dc: Option<HDC>,
    #[cfg(windows)]
    pub background_bitmap: Option<HBITMAP>,
    pub last_selection_rect: Option<SelectionRect>, // 用于计算重绘区域
    // 按钮状态
    pub hover_confirm: bool,        // 确认按钮悬停状态
    pub hover_cancel: bool,         // 取消按钮悬停状态
    #[cfg(windows)]
    pub confirm_rect: Option<RECT>, // 确认按钮区域
    #[cfg(windows)]
    pub cancel_rect: Option<RECT>,  // 取消按钮区域
    // 调整控制点相关
    pub is_resizing: bool,                        // 是否正在调整选区大小
    pub resize_handle: Option<ResizeHandle>,      // 当前拖拽的控制点
    pub hover_handle: Option<ResizeHandle>,       // 当前悬停的控制点
    pub resize_start_point: Option<(i32, i32)>,   // 调整开始时的鼠标位置
    pub resize_start_rect: Option<SelectionRect>, // 调整开始时的选区
    // 基础 BGRA 预乘像素缓冲（屏幕截图），仅初始化一次，用于每帧复制
    pub base_bgra_premul: Option<Vec<u8>>,
    // 半透明黑色 1x1 源，用于 AlphaBlend 叠加蒙版
    #[cfg(windows)]
    pub overlay_dc: Option<HDC>,
    #[cfg(windows)]
    pub overlay_bitmap: Option<HBITMAP>,
    // 离屏合成帧缓冲（与窗口同尺寸），用于一次性呈现，避免闪烁
    #[cfg(windows)]
    pub frame_dc: Option<HDC>,
    #[cfg(windows)]
    pub frame_bitmap: Option<HBITMAP>,
}

impl NativeScreenshotState {
    /// 创建新的截屏状态
    pub fn new(screenshot_image: DynamicImage, screen_width: i32, screen_height: i32) -> Self {
        Self {
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
            #[cfg(windows)]
            background_dc: None,
            #[cfg(windows)]
            background_bitmap: None,
            last_selection_rect: None,
            hover_confirm: false,
            hover_cancel: false,
            #[cfg(windows)]
            confirm_rect: None,
            #[cfg(windows)]
            cancel_rect: None,
            is_resizing: false,
            resize_handle: None,
            hover_handle: None,
            resize_start_point: None,
            resize_start_rect: None,
            base_bgra_premul: None,
            #[cfg(windows)]
            overlay_dc: None,
            #[cfg(windows)]
            overlay_bitmap: None,
            #[cfg(windows)]
            frame_dc: None,
            #[cfg(windows)]
            frame_bitmap: None,
        }
    }
}

/// 资源清理（包含背景、叠加源与离屏帧缓冲）
#[cfg(windows)]
pub unsafe fn cleanup_background_resources() {
    use windows::Win32::Graphics::Gdi::{DeleteDC, DeleteObject};
    
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
pub unsafe fn reset_global_state() {
    let mut state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
    *state_guard = None;
}
