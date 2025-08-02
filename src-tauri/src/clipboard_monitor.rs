use arboard::Clipboard;
use once_cell::sync::Lazy;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::clipboard_content::image_to_data_url;
use crate::clipboard_history;
use crate::image_manager::get_image_manager;

// 监听器控制状态
static MONITOR_RUNNING: AtomicBool = AtomicBool::new(false);
static LAST_CLIPBOARD_CONTENT: Lazy<Arc<Mutex<String>>> =
    Lazy::new(|| Arc::new(Mutex::new(String::new())));

// 粘贴状态计数器 - 用于区分真正的复制和粘贴过程中的剪贴板设置
// 使用计数器而不是布尔值，避免并发粘贴操作的竞态条件
static PASTING_COUNT: AtomicUsize = AtomicUsize::new(0);

// 上次忽略的缓存文件路径 - 避免重复检测相同的缓存文件
static LAST_IGNORED_CACHE_FILES: Lazy<Arc<Mutex<Vec<String>>>> =
    Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

// 启动剪贴板监听器
pub fn start_clipboard_monitor(app_handle: AppHandle) {
    // 如果已经在运行，直接返回
    if MONITOR_RUNNING.load(Ordering::Relaxed) {
        println!("剪贴板监听器已在运行");
        return;
    }

    MONITOR_RUNNING.store(true, Ordering::Relaxed);
    println!("启动剪贴板监听器");

    // 在后台线程中运行监听循环
    thread::spawn(move || {
        clipboard_monitor_loop(app_handle);
    });
}

// 停止剪贴板监听器
pub fn stop_clipboard_monitor() {
    MONITOR_RUNNING.store(false, Ordering::Relaxed);
    println!("停止剪贴板监听器");
}

// 剪贴板监听循环
fn clipboard_monitor_loop(app_handle: AppHandle) {
    let mut clipboard = match Clipboard::new() {
        Ok(cb) => cb,
        Err(e) => {
            println!("创建剪贴板实例失败: {}", e);
            MONITOR_RUNNING.store(false, Ordering::Relaxed);
            return;
        }
    };

    while MONITOR_RUNNING.load(Ordering::Relaxed) {
        // 检查剪贴板监听是否被禁用
        if !clipboard_history::is_monitoring_enabled() {
            thread::sleep(Duration::from_millis(200));
            continue;
        }

        // 尝试获取剪贴板内容
        let current_content = get_clipboard_content(&mut clipboard);

        if let Some(content) = current_content {
            // 检查内容是否发生变化
            let mut last_content = LAST_CLIPBOARD_CONTENT.lock().unwrap();
            if *last_content != content {
                // println!("检测到剪贴板内容变化");

                // 更新最后的内容
                *last_content = content.clone();
                drop(last_content); // 释放锁

                // 添加到历史记录，并检查是否真的添加了新内容
                // 如果正在粘贴，不移动重复内容的位置
                let move_duplicates = !is_pasting_internal();
                let was_added =
                    clipboard_history::add_to_history_with_check_and_move(content, move_duplicates);

                // 只有在真正添加了新内容且非粘贴状态下才播放复制音效
                if was_added && !is_pasting_internal() {
                    crate::sound_manager::play_copy_sound();
                }

                // 发射事件通知前端
                if let Err(e) = app_handle.emit("clipboard-changed", ()) {
                    println!("发射剪贴板变化事件失败: {}", e);
                }
            }
        }

        // 等待一段时间再检查
        thread::sleep(Duration::from_millis(200));
    }

    println!("剪贴板监听循环结束");
}

// 获取剪贴板内容（文本、图片或文件）
fn get_clipboard_content(clipboard: &mut Clipboard) -> Option<String> {
    // 首先尝试获取文件
    if let Ok(file_paths) = crate::file_handler::get_clipboard_files() {
        if !file_paths.is_empty() {
            // 检查是否所有文件都来自图片缓存目录
            let all_from_cache = file_paths.iter().all(|path| is_from_image_cache(path));

            // 如果所有文件都来自图片缓存目录，检查是否与上次相同
            if all_from_cache {
                let mut last_ignored = LAST_IGNORED_CACHE_FILES.lock().unwrap();

                // 检查文件路径是否与上次相同
                let paths_changed = last_ignored.len() != file_paths.len()
                    || !file_paths.iter().all(|path| last_ignored.contains(path));

                if paths_changed {
                    // 文件路径发生了变化，更新记录
                    *last_ignored = file_paths.clone();
                }
                // 无论是否变化，都忽略这次剪贴板变化
                return None;
            } else {
                // 不是缓存文件，清空上次忽略的记录
                let mut last_ignored = LAST_IGNORED_CACHE_FILES.lock().unwrap();
                last_ignored.clear();
            }

            // 处理文件列表
            let mut file_infos = Vec::new();
            for path in &file_paths {
                if let Ok(file_info) = crate::file_handler::get_file_info(path) {
                    file_infos.push(file_info);
                }
            }

            if !file_infos.is_empty() {
                let file_data = crate::file_handler::FileClipboardData {
                    files: file_infos,
                    operation: "copy".to_string(),
                };

                // 序列化文件数据
                if let Ok(json_str) = serde_json::to_string(&file_data) {
                    return Some(format!("files:{}", json_str));
                }
            }
        }
    }

    // 然后尝试获取文本
    if let Ok(text) = clipboard.get_text() {
        // 过滤空白内容：检查去除空白字符后是否为空
        if !text.is_empty() && !text.trim().is_empty() {
            return Some(text);
        }
    }

    // 最后尝试获取图片
    if clipboard_history::is_save_images() {
        if let Ok(img) = clipboard.get_image() {
            // 将图片转换为 data URL
            let data_url = image_to_data_url(&img);

            // 尝试使用图片管理器保存图片
            if let Ok(image_manager) = get_image_manager() {
                if let Ok(manager) = image_manager.lock() {
                    match manager.save_image(&data_url) {
                        Ok(image_info) => {
                            // 返回图片引用而不是完整的data URL
                            return Some(format!("image:{}", image_info.id));
                        }
                        Err(e) => {
                            println!("保存图片失败: {}, 使用原始data URL", e);
                            return Some(data_url);
                        }
                    }
                }
            }

            // 如果图片管理器不可用，回退到原始方式
            return Some(data_url);
        }
    }

    None
}

// 检查监听器是否正在运行
pub fn is_monitor_running() -> bool {
    MONITOR_RUNNING.load(Ordering::Relaxed)
}

// 开始粘贴操作 - 增加粘贴计数器
pub fn start_pasting_operation() {
    PASTING_COUNT.fetch_add(1, Ordering::Relaxed);
}

// 结束粘贴操作 - 减少粘贴计数器
pub fn end_pasting_operation() {
    PASTING_COUNT.fetch_sub(1, Ordering::Relaxed);
}

// 检查是否正在粘贴（内部使用）
fn is_pasting_internal() -> bool {
    PASTING_COUNT.load(Ordering::Relaxed) > 0
}

// 检查是否正在粘贴（公开接口）
pub fn is_currently_pasting() -> bool {
    PASTING_COUNT.load(Ordering::Relaxed) > 0
}

// 初始化最后的剪贴板内容，避免启动时重复添加
pub fn initialize_last_content(content: String) {
    if let Ok(mut last_content) = LAST_CLIPBOARD_CONTENT.lock() {
        *last_content = content;
    }
}

// 初始化剪贴板状态 - 获取当前剪贴板内容并添加到历史记录，同时初始化监听器状态
pub fn initialize_clipboard_state() {
    if let Ok(mut clipboard) = Clipboard::new() {
        // 使用与监听器相同的逻辑获取剪贴板内容
        if let Some(content) = get_clipboard_content(&mut clipboard) {
            // 过滤空白内容：检查去除空白字符后是否为空
            if !content.trim().is_empty() {
                // 使用与监听器相同的逻辑：检查重复并决定是否添加/移动
                // 初始化时不移动重复内容，只是确保内容在历史记录中
                let _was_added =
                    clipboard_history::add_to_history_with_check_and_move(content.clone(), false);
                // 初始化监听器的最后内容，避免重复添加
                initialize_last_content(content);
            }
        }
    }
}

// 检查文件路径是否来自图片缓存目录
fn is_from_image_cache(file_path: &str) -> bool {
    // 获取图片缓存目录路径
    if let Some(app_data_dir) = dirs::data_local_dir() {
        let cache_dir = app_data_dir.join("quickclipboard").join("clipboard_images");
        if let Ok(cache_path) = cache_dir.canonicalize() {
            if let Ok(file_path_buf) = std::path::Path::new(file_path).canonicalize() {
                return file_path_buf.starts_with(cache_path);
            }
        }
    }
    false
}
