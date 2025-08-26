use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, ImageFormat, GenericImageView};
use super::screenshot_state::{SelectionRect, ResizeHandle};

#[cfg(windows)]
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::{Dwm::*, Gdi::*},
        UI::WindowsAndMessaging::*,
    },
};

/// 获取调整控制点的位置
pub fn get_resize_handles(selection: &SelectionRect) -> Vec<(ResizeHandle, RECT)> {
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
pub fn hit_test_resize_handles(selection: &SelectionRect, x: i32, y: i32) -> Option<ResizeHandle> {
    let handles = get_resize_handles(selection);
    for (handle, rect) in handles { 
        if x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom { 
            return Some(handle); 
        } 
    }
    None
}

/// 根据控制点拖拽更新选区
pub fn update_selection_with_handle(
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
    if new_width < MIN_SIZE { 
        if handle == ResizeHandle::TopLeft || handle == ResizeHandle::BottomLeft || handle == ResizeHandle::LeftCenter { 
            new_x = new_x + new_width - MIN_SIZE; 
        } 
        new_width = MIN_SIZE; 
    }
    if new_height < MIN_SIZE { 
        if handle == ResizeHandle::TopLeft || handle == ResizeHandle::TopCenter || handle == ResizeHandle::TopRight { 
            new_y = new_y + new_height - MIN_SIZE; 
        } 
        new_height = MIN_SIZE; 
    }
    
    SelectionRect { 
        x: new_x, 
        y: new_y, 
        width: new_width as u32, 
        height: new_height as u32 
    }
}

/// 计算工具栏的最佳位置，避免超出屏幕
pub fn calculate_toolbar_position(
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
        let final_x = if preferred_x < screen_left + margin { 
            screen_left + margin 
        } else if preferred_x + toolbar_width > virtual_right - margin { 
            virtual_right - toolbar_width - margin 
        } else { 
            preferred_x 
        };
        return (final_x, preferred_y);
    }
    
    let top_y = selection.y - toolbar_height - margin;
    if top_y >= screen_top + margin {
        let final_x = if preferred_x < screen_left + margin { 
            screen_left + margin 
        } else if preferred_x + toolbar_width > virtual_right - margin { 
            virtual_right - toolbar_width - margin 
        } else { 
            preferred_x 
        };
        return (final_x, top_y);
    }
    
    let right_x = selection.x + selection.width as i32 + margin;
    if right_x + toolbar_width <= virtual_right - margin {
        let center_y = selection.y + (selection.height as i32 - toolbar_height) / 2;
        let final_y = if center_y < screen_top + margin { 
            screen_top + margin 
        } else if center_y + toolbar_height > virtual_bottom - margin { 
            virtual_bottom - toolbar_height - margin 
        } else { 
            center_y 
        };
        return (right_x, final_y);
    }
    
    let left_x = selection.x - toolbar_width - margin;
    if left_x >= screen_left + margin {
        let center_y = selection.y + (selection.height as i32 - toolbar_height) / 2;
        let final_y = if center_y < screen_top + margin { 
            screen_top + margin 
        } else if center_y + toolbar_height > virtual_bottom - margin { 
            virtual_bottom - toolbar_height - margin 
        } else { 
            center_y 
        };
        return (left_x, final_y);
    }
    
    (virtual_right - toolbar_width - margin, virtual_bottom - toolbar_height - margin)
}

/// 智能选区：根据鼠标位置自动捕捉到下方窗口/子窗口矩形
#[cfg(windows)]
pub unsafe fn get_auto_selection_at_point(hwnd_overlay: HWND, x: i32, y: i32) -> Option<SelectionRect> {
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
#[repr(C)]
pub struct EnumFindData {
    pub point: POINT,
    pub overlay: HWND,
    pub found: HWND,
}

#[cfg(windows)]
pub unsafe extern "system" fn enum_windows_find_under_point(hwnd: HWND, lparam: LPARAM) -> BOOL {
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

/// 裁剪并保存到剪贴板（与旧版一致）
#[cfg(windows)]
pub fn crop_and_save_to_clipboard(
    image: &DynamicImage,
    selection: &SelectionRect,
) -> std::result::Result<(), String> {
    if selection.width == 0 || selection.height == 0 { 
        return Err("选区尺寸无效".to_string()); 
    }
    let (img_width, img_height) = image.dimensions();
    if selection.x < 0 || selection.y < 0 || selection.x as u32 + selection.width > img_width || selection.y as u32 + selection.height > img_height { 
        return Err("选区超出图像范围".to_string()); 
    }
    let cropped = image.crop_imm(selection.x as u32, selection.y as u32, selection.width, selection.height);
    let quality = crate::settings::get_global_settings().screenshot_quality;
    let data_url = image_to_data_url_with_quality(&cropped, quality)?;
    use crate::clipboard_content::set_clipboard_content_no_history;
    set_clipboard_content_no_history(data_url).map_err(|e| format!("保存截屏到剪贴板失败: {}", e))?;
    Ok(())
}

/// 根据质量设置将图像转换为data URL
pub fn image_to_data_url_with_quality(
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
