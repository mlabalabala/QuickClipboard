use crate::image_manager::{get_image_manager, ImageInfo};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    RwLock,
};

pub const DEFAULT_HISTORY_SIZE: usize = 50;

// 可配置的历史记录数量限制
pub static HISTORY_LIMIT: Lazy<RwLock<usize>> = Lazy::new(|| RwLock::new(DEFAULT_HISTORY_SIZE));

// 持久化文件路径
pub static HISTORY_FILE: Lazy<PathBuf> = Lazy::new(|| {
    // 优先使用系统本地数据目录，否则退回临时目录
    let mut dir = dirs::data_local_dir().unwrap_or_else(|| std::env::temp_dir());
    dir.push("quickclipboard");
    // 确保目录存在
    let _ = std::fs::create_dir_all(&dir);
    dir.push("history.json");
    dir
});

// 使用Lazy和Mutex来存储剪贴板历史
pub static CLIPBOARD_HISTORY: Lazy<std::sync::Mutex<VecDeque<String>>> =
    Lazy::new(|| std::sync::Mutex::new(VecDeque::with_capacity(DEFAULT_HISTORY_SIZE)));

// 剪贴板监听控制
pub static MONITORING_ENABLED: AtomicBool = AtomicBool::new(true);

// 忽略重复内容控制
pub static IGNORE_DUPLICATES: AtomicBool = AtomicBool::new(true);

// 保存图片控制
pub static SAVE_IMAGES: AtomicBool = AtomicBool::new(true);

// 剪贴板条目数据结构
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: usize,
    pub text: String,
    pub is_image: bool,
    pub image_id: Option<String>, // 如果是图片，存储图片ID
    pub timestamp: u64,           // 添加时间戳
}

impl ClipboardItem {
    pub fn new_text(id: usize, text: String) -> Self {
        Self {
            id,
            text,
            is_image: false,
            image_id: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn new_image(id: usize, image_id: String) -> Self {
        Self {
            id,
            text: format!("image:{}", image_id), // 保持向后兼容
            is_image: true,
            image_id: Some(image_id),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

// 保存历史记录到文件
pub fn save_history(history: &VecDeque<String>) {
    let vec: Vec<String> = history.iter().cloned().collect();
    match serde_json::to_string(&vec) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&*HISTORY_FILE, json) {
                println!("保存历史记录失败: {}", e);
            }
        }
        Err(e) => println!("序列化历史记录失败: {}", e),
    }
}

// 从文件加载历史记录
pub fn load_history() {
    if let Ok(content) = std::fs::read_to_string(&*HISTORY_FILE) {
        if let Ok(list) = serde_json::from_str::<Vec<String>>(&content) {
            let mut history = CLIPBOARD_HISTORY.lock().unwrap();
            history.clear();
            let limit = *HISTORY_LIMIT.read().unwrap();
            for text in list.into_iter().take(limit).rev() {
                // 保持顺序，最新在前
                history.push_front(text);
            }
        }
    }
}

// 监听剪贴板变化并添加到历史记录
pub fn add_to_history(text: String) {
    // 如果监听被禁用，直接返回
    if !MONITORING_ENABLED.load(Ordering::Relaxed) {
        println!("剪贴板监听已禁用，跳过添加历史记录");
        return;
    }

    let mut history = CLIPBOARD_HISTORY.lock().unwrap();

    // 检查是否已存在相同内容
    if let Some(pos) = history.iter().position(|item| item == &text) {
        // 无论是否启用忽略重复内容，都将重复内容移动到最前面
        // 这样用户可以看到反馈，知道复制操作已执行
        let existing_text = history.remove(pos).unwrap();
        history.push_front(existing_text);
        save_history(&history);
        return;
    }

    // 新文本：添加到历史开头
    history.push_front(text);

    // 保持历史记录在最大容量以内
    let limit = *HISTORY_LIMIT.read().unwrap();
    while history.len() > limit {
        history.pop_back();
    }

    // 保存到文件
    save_history(&history);
}

// 添加到历史记录并返回是否真正添加了新内容
pub fn add_to_history_with_check(text: String) -> bool {
    add_to_history_with_check_and_move(text, true)
}

// 添加到历史记录并返回是否真正添加了新内容，可控制是否移动重复内容
pub fn add_to_history_with_check_and_move(text: String, move_duplicates: bool) -> bool {
    // 如果监听被禁用，直接返回
    if !MONITORING_ENABLED.load(Ordering::Relaxed) {
        println!("剪贴板监听已禁用，跳过添加历史记录");
        return false;
    }

    let mut history = CLIPBOARD_HISTORY.lock().unwrap();

    // 检查是否已存在相同内容
    if let Some(pos) = history.iter().position(|item| item == &text) {
        if move_duplicates {
            // 只有在允许移动重复内容时才移动到最前面（复制操作）
            let existing_text = history.remove(pos).unwrap();
            history.push_front(existing_text);
            save_history(&history);
            return true; // 移动了位置，算作添加了新内容
        } else {
            // 不移动重复内容（粘贴操作），直接返回false
            return false;
        }
    }

    // 新文本：添加到历史开头
    history.push_front(text);

    // 保持历史记录在最大容量以内
    let limit = *HISTORY_LIMIT.read().unwrap();
    while history.len() > limit {
        history.pop_back();
    }

    // 保存到文件
    save_history(&history);

    // 返回true表示添加了新内容
    true
}

// 设置历史记录数量限制
pub fn set_history_limit(limit: usize) {
    let mut history_limit = HISTORY_LIMIT.write().unwrap();
    *history_limit = limit;

    // 如果当前历史记录超过新的限制，则截断
    let mut history = CLIPBOARD_HISTORY.lock().unwrap();
    while history.len() > limit {
        history.pop_back();
    }

    // 保存到文件
    save_history(&history);
}

// 重新排序历史记录
pub fn reorder_history(items: Vec<String>) {
    let mut history = CLIPBOARD_HISTORY.lock().unwrap();
    history.clear();

    // 将重新排序的项目添加到历史记录中
    for item in items {
        history.push_back(item);
    }

    // 保存到文件
    save_history(&history);
}

// 设置剪贴板监听状态
pub fn set_monitoring_enabled(enabled: bool) {
    MONITORING_ENABLED.store(enabled, Ordering::Relaxed);
}

// 检查剪贴板监听是否启用
pub fn is_monitoring_enabled() -> bool {
    MONITORING_ENABLED.load(Ordering::Relaxed)
}

// 设置忽略重复内容状态
pub fn set_ignore_duplicates(enabled: bool) {
    IGNORE_DUPLICATES.store(enabled, Ordering::Relaxed);
    println!("忽略重复内容设置: {}", enabled);
}

// 检查是否忽略重复内容
pub fn is_ignore_duplicates_enabled() -> bool {
    IGNORE_DUPLICATES.load(Ordering::Relaxed)
}

// 设置保存图片状态
pub fn set_save_images(enabled: bool) {
    SAVE_IMAGES.store(enabled, Ordering::Relaxed);
    println!("保存图片设置: {}", enabled);
}

// 检查是否保存图片
pub fn is_save_images_enabled() -> bool {
    SAVE_IMAGES.load(Ordering::Relaxed)
}

// 根据索引删除剪贴板项目
pub fn delete_item_by_index(index: usize) -> Result<(), String> {
    let mut history = CLIPBOARD_HISTORY.lock().unwrap();

    if index >= history.len() {
        return Err("索引超出范围".to_string());
    }

    // 获取要删除的项目
    let item = history.get(index).cloned();

    // 如果是图片，删除对应的图片文件
    if let Some(item_text) = item {
        if item_text.starts_with("image:") {
            let image_id = item_text.strip_prefix("image:").unwrap_or("");
            if !image_id.is_empty() {
                match get_image_manager() {
                    Ok(image_manager) => {
                        if let Ok(manager) = image_manager.lock() {
                            if let Err(e) = manager.delete_image(image_id) {
                                println!("删除图片文件失败: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("获取图片管理器失败: {}", e);
                    }
                }
            }
        }
    }

    // 从历史记录中删除
    history.remove(index);

    // 保存到文件
    save_history(&history);

    println!("已删除索引为 {} 的剪贴板项目", index);
    Ok(())
}

// 根据ID删除剪贴板项目（兼容前端调用）
pub fn delete_item_by_id(id: usize) -> Result<(), String> {
    delete_item_by_index(id)
}

// 更新剪贴板项目内容
pub fn update_item_content(index: usize, new_content: String) -> Result<(), String> {
    let mut history = CLIPBOARD_HISTORY.lock().unwrap();

    if index >= history.len() {
        return Err(format!(
            "索引 {} 超出范围，当前历史记录数量: {}",
            index,
            history.len()
        ));
    }

    // 更新指定索引的内容
    if let Some(item) = history.get_mut(index) {
        *item = new_content.clone();

        // 保存到文件
        save_history(&history);

        println!("已更新索引为 {} 的剪贴板项目内容", index);
        Ok(())
    } else {
        Err(format!("无法找到索引为 {} 的剪贴板项目", index))
    }
}

// 清空所有剪贴板历史
pub fn clear_all() -> Result<(), String> {
    let mut history = CLIPBOARD_HISTORY.lock().unwrap();

    // 删除所有图片文件
    for item_text in history.iter() {
        if item_text.starts_with("image:") {
            let image_id = item_text.strip_prefix("image:").unwrap_or("");
            if !image_id.is_empty() {
                match crate::image_manager::get_image_manager() {
                    Ok(image_manager) => {
                        if let Ok(manager) = image_manager.lock() {
                            if let Err(e) = manager.delete_image(image_id) {
                                println!("删除图片文件失败: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("获取图片管理器失败: {}", e);
                    }
                }
            }
        }
    }

    // 清空历史记录
    history.clear();

    // 保存到文件
    save_history(&history);

    println!("已清空所有剪贴板历史记录");
    Ok(())
}
