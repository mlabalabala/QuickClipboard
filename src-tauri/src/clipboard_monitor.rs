use arboard::Clipboard;
use once_cell::sync::Lazy;
use std::sync::{
    atomic::{AtomicBool, Ordering},
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

// 粘贴状态标志 - 用于区分真正的复制和粘贴过程中的剪贴板设置
static IS_PASTING: AtomicBool = AtomicBool::new(false);

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

// 获取剪贴板内容（文本或图片）
fn get_clipboard_content(clipboard: &mut Clipboard) -> Option<String> {
    // 首先尝试获取文本
    if let Ok(text) = clipboard.get_text() {
        if !text.is_empty() {
            return Some(text);
        }
    }

    // 如果没有文本，尝试获取图片
    if clipboard_history::is_save_images_enabled() {
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

// 设置粘贴状态 - 在粘贴操作开始前调用
pub fn set_pasting_state(is_pasting: bool) {
    IS_PASTING.store(is_pasting, Ordering::Relaxed);
}

// 检查是否正在粘贴（内部使用）
fn is_pasting_internal() -> bool {
    IS_PASTING.load(Ordering::Relaxed)
}

// 检查是否正在粘贴（公开接口）
pub fn is_currently_pasting() -> bool {
    IS_PASTING.load(Ordering::Relaxed)
}
