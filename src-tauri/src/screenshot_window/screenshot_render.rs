use super::screenshot_state::{NativeScreenshotState, SelectionRect, ResizeHandle};
use super::screenshot_utils::{get_resize_handles, calculate_toolbar_position};

#[cfg(windows)]
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::WindowsAndMessaging::*,
    },
};

/// 离屏合成：先拷贝背景到 frame_dc，再在 frame_dc 上绘制操作层，最后一次 BitBlt 呈现
#[cfg(windows)]
pub unsafe fn compose_and_present(hwnd: HWND, hdc: HDC, state: &NativeScreenshotState, paint_rect: RECT) {
    if let (Some(bg_dc), Some(frame_dc)) = (state.background_dc, state.frame_dc) {
        let left = paint_rect.left;
        let top = paint_rect.top;
        let width = paint_rect.right - paint_rect.left;
        let height = paint_rect.bottom - paint_rect.top;
        if width <= 0 || height <= 0 { return; }

        let _ = BitBlt(frame_dc, left, top, width, height, bg_dc, left, top, SRCCOPY);
        let _ = IntersectClipRect(frame_dc, left, top, paint_rect.right, paint_rect.bottom);
        draw_operation_layer_to(frame_dc, hwnd, state);
        let _ = SelectClipRgn(frame_dc, HRGN::default());
        let _ = BitBlt(hdc, left, top, width, height, frame_dc, left, top, SRCCOPY);
    }
}

/// 在指定 DC 上绘制操作层（选区与按钮等）
#[cfg(windows)]
pub unsafe fn draw_operation_layer_to(target_dc: HDC, hwnd: HWND, state: &NativeScreenshotState) {
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
        draw_selection_rect(target_dc, sel);
        if state.show_confirm_buttons {
            draw_resize_handles(target_dc, sel, state.hover_handle);
            draw_confirm_buttons(target_dc, hwnd, sel);
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
        draw_help_panel_and_text(target_dc, hwnd, state);
    }
}

/// 绘制带半透明背景的帮助提示；若提示面板位于选区内则不显示
#[cfg(windows)]
pub unsafe fn draw_help_panel_and_text(hdc: HDC, hwnd: HWND, state: &NativeScreenshotState) {
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
    draw_help_text(hdc, 0, 0);
}

/// 绘制操作提示
#[cfg(windows)]
pub unsafe fn draw_help_text(hdc: HDC, width: i32, height: i32) {
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
pub unsafe fn draw_selection_rect(hdc: HDC, selection: &SelectionRect) {
    let rect = RECT { 
        left: selection.x, 
        top: selection.y, 
        right: selection.x + selection.width as i32, 
        bottom: selection.y + selection.height as i32 
    };
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
pub unsafe fn draw_confirm_buttons(hdc: HDC, hwnd: HWND, selection: &SelectionRect) {
    use super::screenshot_state::NATIVE_SCREENSHOT_STATE;
    
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

    let (toolbar_x, toolbar_y) = calculate_toolbar_position(
        selection,
        toolbar_width,
        toolbar_height,
        screen_width,
        screen_height,
        screen_left,
        screen_top,
    );

    // 背景：深色、圆角、细边框
    let toolbar_rect = RECT { 
        left: toolbar_x, 
        top: toolbar_y, 
        right: toolbar_x + toolbar_width, 
        bottom: toolbar_y + toolbar_height 
    };
    draw_toolbar_background(hdc, &toolbar_rect, radius_bg, border_thin);

    // 按钮位置
    let confirm_rect = RECT { 
        left: toolbar_x + toolbar_padding, 
        top: toolbar_y + toolbar_padding, 
        right: toolbar_x + toolbar_padding + button_width, 
        bottom: toolbar_y + toolbar_padding + button_height 
    };
    let cancel_rect = RECT { 
        left: confirm_rect.right + button_spacing, 
        top: confirm_rect.top, 
        right: confirm_rect.right + button_spacing + button_width, 
        bottom: confirm_rect.bottom 
    };

    // 悬停状态
    let is_hover_confirm = if let Some(state) = NATIVE_SCREENSHOT_STATE.lock().unwrap().as_ref() { 
        state.hover_confirm 
    } else { 
        false 
    };
    let is_hover_cancel = if let Some(state) = NATIVE_SCREENSHOT_STATE.lock().unwrap().as_ref() { 
        state.hover_cancel 
    } else { 
        false 
    };

    // 绘制按钮（现代配色）
    draw_button(hdc, &confirm_rect, "✓确认", true, is_hover_confirm, radius_btn, border_thin);
    draw_button(hdc, &cancel_rect, "✗取消", false, is_hover_cancel, radius_btn, border_thin);

    // 更新全局状态中的按钮区域
    if let Some(state) = NATIVE_SCREENSHOT_STATE.lock().unwrap().as_mut() {
        let state_mut = std::sync::Arc::make_mut(state);
        state_mut.confirm_rect = Some(confirm_rect);
        state_mut.cancel_rect = Some(cancel_rect);
    }
}

/// 计算工具栏矩形，供区域失效使用（DPI自适应）
#[cfg(windows)]
pub fn compute_toolbar_rect(selection: &SelectionRect, hwnd: HWND) -> RECT {
    unsafe {
        let hdc = GetDC(hwnd);
        let dpi = if hdc.is_invalid() { 
            96 
        } else { 
            let v = GetDeviceCaps(hdc, LOGPIXELSX); 
            let _ = ReleaseDC(hwnd, hdc); 
            if v > 0 { v } else { 96 } 
        };
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

        let (toolbar_x, toolbar_y) = calculate_toolbar_position(
            selection,
            toolbar_width,
            toolbar_height,
            screen_width,
            screen_height,
            screen_left,
            screen_top,
        );
        RECT { 
            left: toolbar_x, 
            top: toolbar_y, 
            right: toolbar_x + toolbar_width, 
            bottom: toolbar_y + toolbar_height 
        }
    }
}

/// 绘制工具栏背景（现代风格）
#[cfg(windows)]
pub unsafe fn draw_toolbar_background(hdc: HDC, rect: &RECT, radius: i32, border_thin: i32) {
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
pub unsafe fn draw_button(hdc: HDC, rect: &RECT, text: &str, is_confirm: bool, is_hover: bool, radius: i32, border_thin: i32) {
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

/// 绘制调整控制点
#[cfg(windows)]
pub unsafe fn draw_resize_handles(
    hdc: HDC,
    selection: &SelectionRect,
    hover_handle: Option<ResizeHandle>,
) {
    let handles = get_resize_handles(selection);
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
