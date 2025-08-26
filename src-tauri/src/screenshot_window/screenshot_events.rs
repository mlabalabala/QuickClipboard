use super::screenshot_state::{
    NATIVE_SCREENSHOT_STATE, SHOULD_EXIT_MESSAGE_LOOP, SelectionRect, ResizeHandle
};
use super::screenshot_utils::{
    hit_test_resize_handles, update_selection_with_handle, get_auto_selection_at_point,
    crop_and_save_to_clipboard
};
use super::screenshot_render::compute_toolbar_rect;
use std::sync::Arc;

#[cfg(windows)]
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, VK_ESCAPE, VK_RETURN},
        UI::WindowsAndMessaging::*,
    },
};

// 自定义消息常量
#[cfg(windows)]
pub const WM_CONFIRM_SCREENSHOT: u32 = WM_USER + 1;

/// 处理鼠标按下事件
#[cfg(windows)]
pub unsafe fn handle_mouse_down(hwnd: HWND, lparam: LPARAM) {
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
                        resize_handle_clicked = hit_test_resize_handles(selection, x, y);
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

/// 处理鼠标移动事件
#[cfg(windows)]
pub unsafe fn handle_mouse_move(hwnd: HWND, lparam: LPARAM) {
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
                                let prev = RECT { 
                                    left: current_rect.x, 
                                    top: current_rect.y, 
                                    right: current_rect.x + current_rect.width as i32, 
                                    bottom: current_rect.y + current_rect.height as i32 
                                };
                                let next = RECT { 
                                    left: new_rect.x, 
                                    top: new_rect.y, 
                                    right: new_rect.x + new_rect.width as i32, 
                                    bottom: new_rect.y + new_rect.height as i32 
                                };
                                let mut left = prev.left.min(next.left) - margin;
                                let mut top = prev.top.min(next.top) - margin;
                                let mut right = prev.right.max(next.right) + margin;
                                let mut bottom = prev.bottom.max(next.bottom) + margin;
                                
                                if state.show_confirm_buttons {
                                    let prev_toolbar = compute_toolbar_rect(current_rect, hwnd);
                                    let next_toolbar = compute_toolbar_rect(&new_rect, hwnd);
                                    left = left.min(prev_toolbar.left).min(next_toolbar.left) - margin;
                                    top = top.min(prev_toolbar.top).min(next_toolbar.top) - margin;
                                    right = right.max(prev_toolbar.right).max(next_toolbar.right) + margin;
                                    bottom = bottom.max(prev_toolbar.bottom).max(next_toolbar.bottom) + margin;
                                }
                                
                                if left < client.left { left = client.left; }
                                if top < client.top { top = client.top; }
                                if right > client.right { right = client.right; }
                                if bottom > client.bottom { bottom = client.bottom; }
                                if right > left && bottom > top { 
                                    dirty_rect = Some(RECT { left, top, right, bottom }); 
                                }
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
                        let new_rect = update_selection_with_handle(handle, start_rect, start_mouse, (x, y));
                        let prev_rect_opt = state.selection_rect.clone();
                        
                        if let Some(ref current_rect) = prev_rect_opt {
                            if current_rect.x != new_rect.x || current_rect.y != new_rect.y || current_rect.width != new_rect.width || current_rect.height != new_rect.height {
                                should_redraw = true;
                                force_full_invalidate = true;
                                let mut client = RECT::default();
                                let _ = GetClientRect(hwnd, &mut client);
                                let margin: i32 = 32;
                                let prev = RECT { 
                                    left: current_rect.x, 
                                    top: current_rect.y, 
                                    right: current_rect.x + current_rect.width as i32, 
                                    bottom: current_rect.y + current_rect.height as i32 
                                };
                                let next = RECT { 
                                    left: new_rect.x, 
                                    top: new_rect.y, 
                                    right: new_rect.x + new_rect.width as i32, 
                                    bottom: new_rect.y + new_rect.height as i32 
                                };
                                let mut left = prev.left.min(next.left) - margin;
                                let mut top = prev.top.min(next.top) - margin;
                                let mut right = prev.right.max(next.right) + margin;
                                let mut bottom = prev.bottom.max(next.bottom) + margin;
                                
                                if state.show_confirm_buttons {
                                    let prev_toolbar = compute_toolbar_rect(current_rect, hwnd);
                                    let next_toolbar = compute_toolbar_rect(&new_rect, hwnd);
                                    left = left.min(prev_toolbar.left).min(next_toolbar.left) - margin;
                                    top = top.min(prev_toolbar.top).min(next_toolbar.top) - margin;
                                    right = right.max(prev_toolbar.right).max(next_toolbar.right) + margin;
                                    bottom = bottom.max(prev_toolbar.bottom).max(next_toolbar.bottom) + margin;
                                }
                                
                                if left < client.left { left = client.left; }
                                if top < client.top { top = client.top; }
                                if right > client.right { right = client.right; }
                                if bottom > client.bottom { bottom = client.bottom; }
                                if right > left && bottom > top { 
                                    dirty_rect = Some(RECT { left, top, right, bottom }); 
                                }
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
                    if let Some(auto_rect) = get_auto_selection_at_point(hwnd, x, y) {
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
                    
                    if let Some(ref confirm_rect) = state.confirm_rect { 
                        if x >= confirm_rect.left && x <= confirm_rect.right && y >= confirm_rect.top && y <= confirm_rect.bottom { 
                            new_hover_confirm = true; 
                        } 
                    }
                    if let Some(ref cancel_rect) = state.cancel_rect { 
                        if x >= cancel_rect.left && x <= cancel_rect.right && y >= cancel_rect.top && y <= cancel_rect.bottom { 
                            new_hover_cancel = true; 
                        } 
                    }
                    if !new_hover_confirm && !new_hover_cancel { 
                        if let Some(ref selection) = state.selection_rect { 
                            new_hover_handle = hit_test_resize_handles(selection, x, y); 
                        } 
                    }
                    
                    if state.hover_confirm != new_hover_confirm || state.hover_cancel != new_hover_cancel || state.hover_handle != new_hover_handle { 
                        state.hover_confirm = new_hover_confirm; 
                        state.hover_cancel = new_hover_cancel; 
                        state.hover_handle = new_hover_handle; 
                        hover_changed = true; 
                    }
                }
            }
        }
    }
    
    if force_full_invalidate { 
        let _ = InvalidateRect(hwnd, None, FALSE); 
    } else if let Some(rect) = dirty_rect { 
        let _ = InvalidateRect(hwnd, Some(&rect), FALSE); 
    } else if should_redraw || hover_changed { 
        let _ = InvalidateRect(hwnd, None, FALSE); 
    }
}

/// 处理鼠标抬起事件
#[cfg(windows)]
pub unsafe fn handle_mouse_up(hwnd: HWND, _lparam: LPARAM) {
    let mut state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
    if let Some(ref mut state_arc) = *state_guard {
        if let Some(state) = Arc::get_mut(state_arc) {
            if state.is_selecting {
                state.is_selecting = false;
                if let Some(selection) = &state.selection_rect { 
                    if selection.width > 10 && selection.height > 10 { 
                        state.show_confirm_buttons = true; 
                        let _ = InvalidateRect(hwnd, None, FALSE); 
                    } 
                }
            } else if state.is_resizing {
                state.is_resizing = false; 
                state.resize_handle = None; 
                state.resize_start_point = None; 
                state.resize_start_rect = None;
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

/// 处理确认截屏
#[cfg(windows)]
pub unsafe fn handle_confirm(hwnd: HWND) {
    let selection_data = {
        let state_guard = NATIVE_SCREENSHOT_STATE.lock().unwrap();
        if let Some(ref state_arc) = *state_guard {
            if let Some(ref selection) = state_arc.selection_rect {
                if selection.width > 10 && selection.height > 10 { 
                    Some((state_arc.screenshot_image.clone(), selection.clone())) 
                } else { 
                    None 
                }
            } else { 
                None 
            }
        } else { 
            None 
        }
    };
    let _ = DestroyWindow(hwnd);
    if let Some((image, selection)) = selection_data {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let _ = crop_and_save_to_clipboard(&image, &selection);
    }
}

/// 统一在 WM_SETCURSOR 中设置光标，避免闪烁
#[cfg(windows)]
pub unsafe fn handle_set_cursor(hwnd: HWND) {
    let mut pt_screen = POINT::default();
    if !GetCursorPos(&mut pt_screen).is_ok() { return; }
    let mut pt_client = pt_screen;
    let _ = ScreenToClient(hwnd, &mut pt_client);

    let state_arc = { 
        let s = NATIVE_SCREENSHOT_STATE.lock().unwrap(); 
        s.clone() 
    };
    if let Some(state) = state_arc {
        if state.is_resizing {
            if let Some(handle) = state.resize_handle { 
                set_cursor_for_handle(handle); 
                return; 
            }
        }
        if state.show_confirm_buttons && !state.is_selecting && !state.is_resizing {
            if let Some(r) = state.confirm_rect {
                if pt_client.x >= r.left && pt_client.x <= r.right && pt_client.y >= r.top && pt_client.y <= r.bottom {
                    if let Ok(cur) = LoadCursorW(None, IDC_HAND) { 
                        SetCursor(cur); 
                        return; 
                    }
                }
            }
            if let Some(r) = state.cancel_rect {
                if pt_client.x >= r.left && pt_client.x <= r.right && pt_client.y >= r.top && pt_client.y <= r.bottom {
                    if let Ok(cur) = LoadCursorW(None, IDC_HAND) { 
                        SetCursor(cur); 
                        return; 
                    }
                }
            }
            if let Some(ref sel) = state.selection_rect {
                if let Some(handle) = hit_test_resize_handles(sel, pt_client.x, pt_client.y) {
                    set_cursor_for_handle(handle);
                    return;
                }
            }
        }
    }
    if let Ok(cur) = LoadCursorW(None, IDC_CROSS) { 
        SetCursor(cur); 
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
    if let Ok(cursor) = LoadCursorW(None, cursor_id) { 
        SetCursor(cursor); 
    }
}

/// 右键处理：先清空选区/状态，再决定是否关闭
#[cfg(windows)]
pub unsafe fn handle_right_button(hwnd: HWND) {
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
    if should_close { 
        let _ = DestroyWindow(hwnd); 
    } else { 
        let _ = InvalidateRect(hwnd, None, FALSE); 
    }
}
