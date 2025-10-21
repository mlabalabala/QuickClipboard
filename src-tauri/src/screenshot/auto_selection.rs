use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use std::collections::HashMap;
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use windows::Win32::Foundation::{POINT, HWND, RECT, BOOL, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, IsWindowVisible, IsIconic, GetWindowRect,
};
use serde::Serialize;
use uiautomation::{UIAutomation, UIElement, UITreeWalker};
use rstar::{RTree, AABB, RTreeObject, PointDistance};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DetectionMode {
    None,
    Window,
    All,
}

impl DetectionMode {
    pub fn from_string(s: &str) -> Self {
        match s {
            "none" => DetectionMode::None,
            "window" => DetectionMode::Window,
            "all" => DetectionMode::All,
            _ => DetectionMode::All,
        }
    }
}

pub struct AutoSelectionManager {
    is_active: Arc<AtomicBool>,
    app_handle: Arc<Mutex<Option<AppHandle>>>,
    screenshot_hwnd: Arc<Mutex<Option<isize>>>,
    cache: Arc<Mutex<HashMap<isize, RTree<CachedElement>>>>,
    thread_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ElementBounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ElementHierarchy {
    pub hierarchy: Vec<ElementBounds>,
    pub current_index: usize,
}

#[derive(Debug, Clone)]
struct CachedElement {
    rect: RECT,
    area: i32,
}

impl RTreeObject for CachedElement {
    type Envelope = AABB<[i32; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_corners(
            [self.rect.left, self.rect.top],
            [self.rect.right, self.rect.bottom]
        )
    }
}

impl PointDistance for CachedElement {
    fn distance_2(&self, point: &[i32; 2]) -> i32 {
        let x = point[0];
        let y = point[1];

        if x >= self.rect.left && x <= self.rect.right 
            && y >= self.rect.top && y <= self.rect.bottom {
            return 0;
        }

        let dx = if x < self.rect.left {
            self.rect.left - x
        } else if x > self.rect.right {
            x - self.rect.right
        } else {
            0
        };
        
        let dy = if y < self.rect.top {
            self.rect.top - y
        } else if y > self.rect.bottom {
            y - self.rect.bottom
        } else {
            0
        };
        
        dx * dx + dy * dy
    }
}

struct EnumData {
    point: POINT,
    exclude_hwnd: Option<isize>,
    found_hwnd: Option<HWND>,
}

impl AutoSelectionManager {
    pub fn new() -> Self {
        Self {
            is_active: Arc::new(AtomicBool::new(false)),
            app_handle: Arc::new(Mutex::new(None)),
            screenshot_hwnd: Arc::new(Mutex::new(None)),
            cache: Arc::new(Mutex::new(HashMap::new())),
            thread_handle: Arc::new(Mutex::new(None)),
        }
    }

    #[inline]
    fn to_css_bounds(bounds: &ElementBounds, scale_factor: f64) -> ElementBounds {
        ElementBounds {
            x: (bounds.x as f64 / scale_factor).round() as i32,
            y: (bounds.y as f64 / scale_factor).round() as i32,
            width: (bounds.width as f64 / scale_factor).round() as i32,
            height: (bounds.height as f64 / scale_factor).round() as i32,
        }
    }

    // 发送元素层级到前端（使用 try_lock 避免死锁）
    #[inline]
    fn emit_hierarchy(app_handle: &Arc<Mutex<Option<AppHandle>>>, bounds: &ElementBounds) {
        if let Some(app_guard) = app_handle.try_lock() {
            if let Some(app) = app_guard.as_ref() {
                if let Some(window) = app.get_webview_window("screenshot") {
                    let scale_factor = window.scale_factor().unwrap_or(1.0);
                    let css_bounds = Self::to_css_bounds(bounds, scale_factor);
                    let hierarchy = ElementHierarchy {
                        hierarchy: vec![css_bounds],
                        current_index: 0,
                    };
                    let _ = window.emit("auto-selection-hierarchy", &hierarchy);
                }
            }
        }
    }

    // 启动自动选区检测
    pub fn start(&self, app: AppHandle) -> Result<(), String> {
        if let Some(handle) = self.thread_handle.lock().take() {
            let _ = handle.join();
        }

        if self.is_active.load(Ordering::Relaxed) {
            return Ok(());
        }

        let screenshot_hwnd = if let Some(window) = app.get_webview_window("screenshot") {
            window.hwnd().ok().map(|h| h.0 as isize)
        } else {
            None
        };

        *self.screenshot_hwnd.lock() = screenshot_hwnd;
        *self.app_handle.lock() = Some(app);
        self.is_active.store(true, Ordering::Relaxed);

        let is_active = Arc::clone(&self.is_active);
        let app_handle = Arc::clone(&self.app_handle);
        let screenshot_hwnd = Arc::clone(&self.screenshot_hwnd);
        let cache = Arc::clone(&self.cache);

        let handle = thread::Builder::new()
            .name("auto-selection".to_string())
            .spawn(move || {
                #[cfg(target_os = "windows")]
                unsafe {
                    use windows::Win32::System::Threading::{
                        GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_ABOVE_NORMAL
                    };
                    let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_ABOVE_NORMAL);
                }
                
                let _ = Self::detection_loop(is_active, app_handle, screenshot_hwnd, cache);
            })
            .expect("创建检测线程失败");

        *self.thread_handle.lock() = Some(handle);

        Ok(())
    }

    // 停止自动选区检测
    pub fn stop(&self) {
        self.is_active.store(false, Ordering::Relaxed);
        *self.app_handle.lock() = None;
        *self.screenshot_hwnd.lock() = None;
        
        if let Some(handle) = self.thread_handle.lock().take() {
            thread::spawn(move || {
                let _ = handle.join();
            });
        }
    }

    // 清除缓存
    pub fn clear_cache(&self) {
        self.cache.lock().clear();
    }

    // 主检测循环
    fn detection_loop(
        is_active: Arc<AtomicBool>,
        app_handle: Arc<Mutex<Option<AppHandle>>>,
        screenshot_hwnd: Arc<Mutex<Option<isize>>>,
        cache: Arc<Mutex<HashMap<isize, RTree<CachedElement>>>>,
    ) -> Result<(), String> {
        let automation = UIAutomation::new()
            .map_err(|e| format!("创建UI Automation失败: {}", e))?;

        let mut last_bounds: Option<ElementBounds> = None;
        let mut last_hwnd: Option<isize> = None;

        while is_active.load(Ordering::Relaxed) {
            let settings = crate::settings::get_global_settings();
            let current_mode = DetectionMode::from_string(&settings.screenshot_element_detection);
            
            if current_mode == DetectionMode::None {
                if last_bounds.is_some() {
                    last_bounds = None;
                    if let Some(app_guard) = app_handle.try_lock() {
                        if let Some(app) = app_guard.as_ref() {
                            if let Some(window) = app.get_webview_window("screenshot") {
                                let _ = window.emit("auto-selection-clear", ());
                            }
                        }
                    }
                }
                thread::sleep(Duration::from_millis(50));
                continue;
            }
            
            let cursor_pos = match crate::mouse_utils::get_cursor_point() {
                Ok(pos) => pos,
                Err(_) => {
                    thread::sleep(Duration::from_millis(1));
                    continue;
                }
            };

            let exclude_hwnd = screenshot_hwnd.lock().clone();
            let target_hwnd = Self::get_window_under_point(cursor_pos, exclude_hwnd);

            if target_hwnd.is_none() {
                if last_bounds.is_some() {
                    last_bounds = None;
                    if let Some(app_guard) = app_handle.try_lock() {
                        if let Some(app) = app_guard.as_ref() {
                            if let Some(window) = app.get_webview_window("screenshot") {
                                let _ = window.emit("auto-selection-clear", ());
                            }
                        }
                    }
                }
                thread::sleep(Duration::from_millis(1));
                continue;
            }

            let hwnd = target_hwnd.unwrap();
            let hwnd_value = hwnd.0 as isize;
            let window_changed = last_hwnd != Some(hwnd_value);
            let is_cached = cache.lock().contains_key(&hwnd_value);
            
            if !is_cached {
                match Self::stream_cache_and_find(&automation, hwnd, cursor_pos, &app_handle, &mut last_bounds, current_mode, &is_active) {
                    Ok(rtree) => {
                        if is_active.load(Ordering::Relaxed) {
                            cache.lock().insert(hwnd_value, rtree);
                        }
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                }
                thread::sleep(Duration::from_millis(1));
                continue;
            }
            
            if window_changed {
                last_bounds = None;
            }
            
            last_hwnd = Some(hwnd_value);

            let cached_rtree = cache.lock().get(&hwnd_value).cloned();
            if let Some(rtree) = cached_rtree {
                match Self::find_element_hierarchy_from_rtree(&rtree, cursor_pos) {
                    Ok(Some(hierarchy_bounds)) => {
                        let smallest_bounds = &hierarchy_bounds[0];
                        let should_update = match &last_bounds {
                            Some(last) => {
                                smallest_bounds.x != last.x
                                    || smallest_bounds.y != last.y
                                    || smallest_bounds.width != last.width
                                    || smallest_bounds.height != last.height
                            }
                            None => true,
                        };

                        if should_update {
                            if let Some(app_guard) = app_handle.try_lock() {
                                if let Some(app) = app_guard.as_ref() {
                                    if let Some(window) = app.get_webview_window("screenshot") {
                                        let scale_factor = window.scale_factor().unwrap_or(1.0);
                                        let css_hierarchy: Vec<ElementBounds> = hierarchy_bounds.iter()
                                            .map(|bounds| Self::to_css_bounds(bounds, scale_factor))
                                            .collect();
                                        
                                        let hierarchy = ElementHierarchy {
                                            hierarchy: css_hierarchy,
                                            current_index: 0,
                                        };
                                        
                                        let _ = window.emit("auto-selection-hierarchy", &hierarchy);
                                    }
                                }
                            }

                            last_bounds = Some(smallest_bounds.clone());
                        }
                    }
                    Ok(None) => {
                        if last_bounds.is_some() {
                            last_bounds = None;
                            if let Some(app_guard) = app_handle.try_lock() {
                                if let Some(app) = app_guard.as_ref() {
                                    if let Some(window) = app.get_webview_window("screenshot") {
                                        let _ = window.emit("auto-selection-clear", ());
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {}
                }
            }

            thread::sleep(Duration::from_micros(500));
        }

        Ok(())
    }

    fn get_window_under_point(point: POINT, exclude_hwnd: Option<isize>) -> Option<HWND> {
        let mut data = EnumData {
            point,
            exclude_hwnd,
            found_hwnd: None,
        };

        unsafe {
            let _ = EnumWindows(
                Some(Self::enum_windows_proc),
                LPARAM(&mut data as *mut _ as isize),
            );
        }

        data.found_hwnd
    }

    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let data = &mut *(lparam.0 as *mut EnumData);

        if let Some(exclude) = data.exclude_hwnd {
            if hwnd.0 == exclude {
                return BOOL(1);
            }
        }

        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return BOOL(1);
        }

        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() {
            return BOOL(1);
        }

        if data.point.x >= rect.left
            && data.point.x <= rect.right
            && data.point.y >= rect.top
            && data.point.y <= rect.bottom
        {
            data.found_hwnd = Some(hwnd);
            return BOOL(0);
        }

        BOOL(1)
    }

    // 流式缓存：TreeWalker 快速定位 + find_all 完整缓存
    fn stream_cache_and_find(
        automation: &UIAutomation,
        target_hwnd: HWND,
        point: POINT,
        app_handle: &Arc<Mutex<Option<AppHandle>>>,
        last_bounds: &mut Option<ElementBounds>,
        detection_mode: DetectionMode,
        is_active: &Arc<AtomicBool>,
    ) -> Result<RTree<CachedElement>, String> {
        if !is_active.load(Ordering::Relaxed) {
            return Err("已停止".to_string());
        }

        let window_element = automation.element_from_handle((target_hwnd.0 as isize).into())
            .map_err(|e| format!("ElementFromHandle 失败: {:?}", e))?;

        let window_rect = window_element.get_bounding_rectangle()
            .map_err(|e| format!("获取窗口边界失败: {:?}", e))?;
        
        let window_left = window_rect.get_left();
        let window_top = window_rect.get_top();
        let window_right = window_rect.get_right();
        let window_bottom = window_rect.get_bottom();
        let window_width = window_right - window_left;
        let window_height = window_bottom - window_top;
        
        // 立即发送窗口边界
        if point.x >= window_left && point.x <= window_right
            && point.y >= window_top && point.y <= window_bottom
        {
            let window_bounds = ElementBounds {
                x: window_left,
                y: window_top,
                width: window_width,
                height: window_height,
            };
            
            Self::emit_hierarchy(app_handle, &window_bounds);
            *last_bounds = Some(window_bounds.clone());
        }

        if detection_mode == DetectionMode::Window {
            let window_area = window_width * window_height;
            let element = CachedElement {
                rect: RECT {
                    left: window_left,
                    top: window_top,
                    right: window_right,
                    bottom: window_bottom,
                },
                area: window_area,
            };
            return Ok(RTree::bulk_load(vec![element]));
        }

        // 快速找到鼠标位置的元素
        let walker = automation.get_raw_view_walker()
            .map_err(|e| format!("获取 TreeWalker 失败: {:?}", e))?;
        
        Self::quick_find_at_point(
            &walker,
            &window_element,
            point,
            app_handle,
            last_bounds,
            is_active,
        );

        if !is_active.load(Ordering::Relaxed) {
            return Err("已停止".to_string());
        }

        // 全量缓存所有元素
        let mut cached = Vec::with_capacity(500);
        
        cached.push(CachedElement {
            rect: RECT {
                left: window_left,
                top: window_top,
                right: window_right,
                bottom: window_bottom,
            },
            area: window_width * window_height,
        });

        let elements = window_element.find_all(
            uiautomation::types::TreeScope::Subtree,
            &automation.create_true_condition()
                .map_err(|e| format!("CreateTrueCondition 失败: {:?}", e))?
        ).map_err(|e| format!("FindAll 失败: {:?}", e))?;

        let mut processed = 0;
        for element in elements.iter() {
            if processed % 100 == 0 && !is_active.load(Ordering::Relaxed) {
                return Err("已停止".to_string());
            }
            processed += 1;

            if let Ok(rect) = element.get_bounding_rectangle() {
                let left = rect.get_left();
                let top = rect.get_top();
                let right = rect.get_right();
                let bottom = rect.get_bottom();
                let width = right - left;
                let height = bottom - top;

                if width >= 1 && height >= 1 && width <= 10000 && height <= 10000 {
                    let area = width * height;
                    
                    if area >= 25 {
                        cached.push(CachedElement { 
                            rect: RECT { left, top, right, bottom },
                            area 
                        });
                    }
                }
            }
        }

        if !is_active.load(Ordering::Relaxed) {
            return Err("已停止".to_string());
        }

        Ok(RTree::bulk_load(cached))
    }

    // 快速查找鼠标位置的元素（深度优先）
    fn quick_find_at_point(
        walker: &UITreeWalker,
        parent: &UIElement,
        point: POINT,
        app_handle: &Arc<Mutex<Option<AppHandle>>>,
        last_bounds: &mut Option<ElementBounds>,
        is_active: &Arc<AtomicBool>,
    ) {
        let mut best_match: Option<ElementBounds> = None;
        let mut stack = vec![parent.clone()];
        let mut checked_count = 0;
        const MAX_QUICK_CHECK: usize = 500;
        
        while let Some(element) = stack.pop() {
            if !is_active.load(Ordering::Relaxed) {
                return;
            }
            
            if checked_count >= MAX_QUICK_CHECK {
                break;
            }
            
            let first_child = match walker.get_first_child(&element) {
                Ok(child) => child,
                Err(_) => continue,
            };
            
            let mut current = Some(first_child);
            
            while let Some(elem) = current {
                checked_count += 1;
                
                let rect = match elem.get_bounding_rectangle() {
                    Ok(r) => r,
                    Err(_) => {
                        current = walker.get_next_sibling(&elem).ok();
                        continue;
                    }
                };
                
                let left = rect.get_left();
                let top = rect.get_top();
                let right = rect.get_right();
                let bottom = rect.get_bottom();
                let width = right - left;
                let height = bottom - top;
                
                if width < 1 || height < 1 || width > 10000 || height > 10000 {
                    current = walker.get_next_sibling(&elem).ok();
                    continue;
                }
                
                let area = width * height;
                
                if area >= 25 {
                    if point.x >= left && point.x <= right && point.y >= top && point.y <= bottom {
                        let should_update = match &best_match {
                            None => true,
                            Some(bounds) => area < (bounds.width * bounds.height),
                        };
                        
                        if should_update {
                            let bounds = ElementBounds {
                                x: left,
                                y: top,
                                width,
                                height,
                            };
                            
                            Self::emit_hierarchy(app_handle, &bounds);
                            *last_bounds = Some(bounds.clone());
                            best_match = Some(bounds);
                        }
                        
                        stack.push(elem.clone());
                    }
                }
                
                current = walker.get_next_sibling(&elem).ok();
            }
        }
    }

    // 从 RTree 缓存中查找元素层级
    fn find_element_hierarchy_from_rtree(
        rtree: &RTree<CachedElement>,
        point: POINT,
    ) -> Result<Option<Vec<ElementBounds>>, String> {
        let query_point = [point.x, point.y];

        let mut matching_elements: Vec<&CachedElement> = rtree
            .locate_all_at_point(&query_point)
            .collect();

        if matching_elements.is_empty() {
            return Ok(None);
        }

        matching_elements.sort_by_key(|element| element.area);

        let hierarchy: Vec<ElementBounds> = matching_elements
            .iter()
            .map(|element| {
                let rect = &element.rect;
                ElementBounds {
                    x: rect.left,
                    y: rect.top,
                    width: rect.right - rect.left,
                    height: rect.bottom - rect.top,
                }
            })
            .collect();

        Ok(Some(hierarchy))
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::Relaxed)
    }
}

use once_cell::sync::Lazy;
pub static AUTO_SELECTION_MANAGER: Lazy<AutoSelectionManager> = Lazy::new(|| AutoSelectionManager::new());

#[tauri::command]
pub fn start_auto_selection(app: AppHandle) -> Result<(), String> {
    AUTO_SELECTION_MANAGER.start(app)
}

#[tauri::command]
pub fn stop_auto_selection() -> Result<(), String> {
    AUTO_SELECTION_MANAGER.stop();
    Ok(())
}

#[tauri::command]
pub fn is_auto_selection_active() -> bool {
    AUTO_SELECTION_MANAGER.is_active()
}

#[tauri::command]
pub fn clear_auto_selection_cache() {
    AUTO_SELECTION_MANAGER.clear_cache();
}
