use arboard::Clipboard;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::Manager;
use tauri::WebviewWindow;

#[cfg(not(debug_assertions))]
use auto_launch::AutoLaunch;

use crate::admin_privileges;
use crate::clipboard_content::{
    image_to_data_url, set_clipboard_content, set_clipboard_content_no_history,
};
use crate::clipboard_history::{self, ClipboardItem, CLIPBOARD_HISTORY};
use crate::groups::{self, Group};
use crate::image_manager::get_image_manager;
use crate::mouse_hook::{disable_mouse_monitoring, enable_mouse_monitoring};
use crate::paste_utils::windows_paste;
use crate::quick_texts::{self, QuickText};
use crate::window_management;

// 全局翻译状态管理
static TRANSLATION_CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Deserialize)]
pub struct GroupParams {
    #[serde(rename = "groupId")]
    pub group_id: String,
}

#[derive(Deserialize)]
pub struct AddToGroupParams {
    pub index: usize,
    #[serde(rename = "groupId")]
    pub group_id: String,
}

#[derive(Deserialize)]
pub struct MoveToGroupParams {
    pub id: String,
    #[serde(rename = "groupId")]
    pub group_id: String,
}

// 从剪贴板获取文本
#[tauri::command]
pub fn get_clipboard_text() -> Result<String, String> {
    match Clipboard::new() {
        Ok(mut clipboard) => match clipboard.get_text() {
            Ok(text) => {
                clipboard_history::add_to_history(text.clone());
                Ok(text)
            }
            Err(_) => Err("剪贴板为空或不是文本格式".into()),
        },
        Err(e) => Err(format!("获取剪贴板失败: {}", e)),
    }
}

// 设置剪贴板文本
#[tauri::command]
pub fn set_clipboard_text(text: String) -> Result<(), String> {
    set_clipboard_content(text)?;
    Ok(())
}

// 设置剪贴板图片
#[tauri::command]
pub fn set_clipboard_image(data_url: String) -> Result<(), String> {
    set_clipboard_content(data_url)?;
    Ok(())
}

// 获取剪贴板历史
#[tauri::command]
pub fn get_clipboard_history() -> Vec<ClipboardItem> {
    let history = CLIPBOARD_HISTORY.lock().unwrap();
    history
        .iter()
        .enumerate()
        .map(|(id, text)| {
            if text.starts_with("data:image/") {
                // 旧格式
                ClipboardItem {
                    id,
                    text: text.clone(),
                    is_image: true,
                    image_id: None,
                    timestamp: 0,
                }
            } else if text.starts_with("image:") {
                // 新格式的图片引用
                let image_id = text.strip_prefix("image:").unwrap_or("").to_string();
                ClipboardItem {
                    id,
                    text: text.clone(),
                    is_image: true,
                    image_id: Some(image_id),
                    timestamp: 0,
                }
            } else {
                // 文本数据
                ClipboardItem {
                    id,
                    text: text.clone(),
                    is_image: false,
                    image_id: None,
                    timestamp: 0,
                }
            }
        })
        .collect()
}

// 刷新剪贴板监听函数，只添加新内容
#[tauri::command]
pub fn refresh_clipboard() -> Result<(), String> {
    match Clipboard::new() {
        Ok(mut clipboard) => {
            if let Ok(text) = clipboard.get_text() {
                if !text.is_empty() {
                    clipboard_history::add_to_history(text);
                    return Ok(());
                }
            }
            // 尝试图片
            match clipboard.get_image() {
                Ok(img) => {
                    let data_url = image_to_data_url(&img);
                    clipboard_history::add_to_history(data_url);
                    Ok(())
                }
                Err(_) => Ok(()),
            }
        }
        Err(e) => Err(format!("获取剪贴板失败: {}", e)),
    }
}

// 切换窗口显示/隐藏状态
#[tauri::command]
pub fn toggle_window_visibility(window: WebviewWindow) -> Result<(), String> {
    if window.is_visible().unwrap_or(true) {
        // 先判断是否固定，如果是固定就取消隐藏
        #[cfg(windows)]
        {
            if window_management::get_window_pinned() {
                // 固定时不隐藏
                return Ok(());
            }
        }
        // 先 show 一下再 hide，强制刷新
        window.show().ok();
        std::thread::sleep(std::time::Duration::from_millis(10));
        window.hide().map_err(|e| format!("隐藏窗口失败: {}", e))?;
        // 隐藏窗口时停止鼠标监听
        #[cfg(windows)]
        disable_mouse_monitoring();
    } else {
        // 智能定位窗口到光标位置
        #[cfg(windows)]
        {
            let _ = window_management::position_window_at_cursor(&window);
        }

        window.show().map_err(|e| format!("显示窗口失败: {}", e))?;
        // 确保窗口设置为超级置顶工具窗口（不抢占焦点，且在开始菜单之上）
        #[cfg(windows)]
        {
            let _ = window_management::set_super_topmost_window(&window);
        }
        // 显示窗口时启用鼠标监听
        #[cfg(windows)]
        enable_mouse_monitoring();
    }
    Ok(())
}

// 保留原有的greet函数以兼容现有代码
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

// 窗口管理功能
#[tauri::command]
pub fn focus_clipboard_window(window: WebviewWindow) -> Result<(), String> {
    window_management::focus_clipboard_window(window)
}

#[tauri::command]
pub fn restore_last_focus() -> Result<(), String> {
    window_management::restore_last_focus()
}

#[tauri::command]
pub fn set_window_pinned(pinned: bool) -> Result<(), String> {
    window_management::set_window_pinned(pinned)
}

// 如果主窗口是自动显示的，则隐藏它
#[tauri::command]
pub fn hide_main_window_if_auto_shown(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    if let Some(main_window) = app.get_webview_window("main") {
        window_management::hide_main_window_if_auto_shown(&main_window)
    } else {
        Err("找不到主窗口".to_string())
    }
}

// =================== 常用文本相关命令 ===================

// 获取所有常用文本
#[tauri::command]
pub fn get_quick_texts() -> Vec<QuickText> {
    quick_texts::get_all_quick_texts()
}

// 添加常用文本
#[tauri::command]
pub fn add_quick_text(
    title: String,
    content: String,
    groupId: String,
) -> Result<QuickText, String> {
    // 直接使用传入的groupId，就像拖拽功能一样
    quick_texts::add_quick_text_with_group(title, content, groupId)
}

// 更新常用文本
#[tauri::command]
pub fn update_quick_text(
    id: String,
    title: String,
    content: String,
    groupId: String,
) -> Result<QuickText, String> {
    // 直接使用传入的groupId，就像拖拽功能一样
    quick_texts::update_quick_text_with_group(id, title, content, Some(groupId))
}

// 删除常用文本
#[tauri::command]
pub fn delete_quick_text(id: String) -> Result<(), String> {
    quick_texts::delete_quick_text(id)
}

// 将剪贴板历史项添加到常用文本
#[tauri::command]
pub fn add_clipboard_to_favorites(index: usize) -> Result<QuickText, String> {
    let history = clipboard_history::CLIPBOARD_HISTORY.lock().unwrap();

    if index >= history.len() {
        return Err(format!("索引 {} 超出历史范围", index));
    }

    let content = history[index].clone();
    drop(history); // 释放锁

    // 处理内容，如果是图片则创建副本
    let final_content = if content.starts_with("image:") {
        // 提取图片ID
        let image_id = content.strip_prefix("image:").unwrap_or("");
        if !image_id.is_empty() {
            // 创建图片副本
            match get_image_manager() {
                Ok(image_manager) => {
                    match image_manager.lock() {
                        Ok(manager) => {
                            match manager.copy_image(image_id) {
                                Ok(new_image_info) => {
                                    format!("image:{}", new_image_info.id)
                                }
                                Err(e) => {
                                    println!("复制图片失败: {}, 使用原始引用", e);
                                    content // 如果复制失败，使用原始引用
                                }
                            }
                        }
                        Err(e) => {
                            println!("获取图片管理器锁失败: {}, 使用原始引用", e);
                            content
                        }
                    }
                }
                Err(e) => {
                    println!("获取图片管理器失败: {}, 使用原始引用", e);
                    content
                }
            }
        } else {
            content
        }
    } else {
        content.clone()
    };

    // 生成标题：根据内容类型生成合适的标题
    let title = if final_content.starts_with("data:image/") || final_content.starts_with("image:") {
        // 图片内容使用固定标题
        "图片".to_string()
    } else if final_content.starts_with("files:") {
        // 文件内容解析文件名作为标题
        generate_files_title(&final_content)
    } else if final_content.len() > 30 {
        // 文本内容取前30个字符作为标题
        format!("{}...", &final_content[..30])
    } else {
        final_content.clone()
    };

    // 添加到常用文本
    quick_texts::add_quick_text(title, final_content)
}

// =================== 鼠标监听控制命令 ===================

// 启用鼠标监听
#[tauri::command]
pub fn enable_mouse_monitoring_command() -> Result<(), String> {
    #[cfg(windows)]
    enable_mouse_monitoring();
    Ok(())
}

// 禁用鼠标监听
#[tauri::command]
pub fn disable_mouse_monitoring_command() -> Result<(), String> {
    #[cfg(windows)]
    disable_mouse_monitoring();
    Ok(())
}

// =================== 设置相关命令 ===================

// 设置开机自启动
#[tauri::command]
pub fn set_startup_launch(enabled: bool) -> Result<(), String> {
    // 在开发模式下跳过开机自启动设置
    #[cfg(debug_assertions)]
    {
        println!("开发模式下跳过开机自启动设置: enabled = {}", enabled);
        return Ok(());
    }

    #[cfg(not(debug_assertions))]
    {
        let app_name = "QuickClipboard";
        let app_path = std::env::current_exe().map_err(|e| format!("获取程序路径失败: {}", e))?;

        let auto_launch = AutoLaunch::new(app_name, &app_path.to_string_lossy(), &[] as &[&str]);

        if enabled {
            auto_launch
                .enable()
                .map_err(|e| format!("启用开机自启动失败: {}", e))?;
        } else {
            auto_launch
                .disable()
                .map_err(|e| format!("禁用开机自启动失败: {}", e))?;
        }

        Ok(())
    }
}

// 设置历史记录数量限制
#[tauri::command]
pub fn set_history_limit(limit: usize) -> Result<(), String> {
    clipboard_history::set_history_limit(limit);
    Ok(())
}

// =================== 拖拽排序相关命令 ===================

// 重新排序剪贴板历史
#[tauri::command]
pub fn reorder_clipboard_history(items: Vec<String>) -> Result<(), String> {
    clipboard_history::reorder_history(items);
    Ok(())
}

// 重新排序常用文本
#[tauri::command]
pub fn reorder_quick_texts(items: Vec<QuickText>) -> Result<(), String> {
    quick_texts::reorder_quick_texts(items)
}

// =================== 分组相关命令 ===================

// 获取所有分组
#[tauri::command]
pub fn get_groups() -> Vec<Group> {
    groups::get_all_groups()
}

// 添加分组
#[tauri::command]
pub fn add_group(name: String, icon: String) -> Result<Group, String> {
    groups::add_group(name, icon)
}

// 更新分组
#[tauri::command]
pub fn update_group(id: String, name: String, icon: String) -> Result<Group, String> {
    groups::update_group(id, name, icon)
}

// 删除分组
#[tauri::command]
pub fn delete_group(id: String) -> Result<(), String> {
    groups::delete_group(id)
}

// 按分组获取常用文本
#[tauri::command]
pub fn get_quick_texts_by_group(group_id: String) -> Vec<QuickText> {
    quick_texts::get_quick_texts_by_group(&group_id)
}

// 移动常用文本到分组
#[tauri::command]
pub fn move_quick_text_to_group(id: String, group_id: String) -> Result<(), String> {
    quick_texts::move_quick_text_to_group(id, group_id)
}

// 打开设置窗口
#[tauri::command]
pub async fn open_settings_window(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    // 检查设置窗口是否已经存在
    if let Some(settings_window) = app.get_webview_window("settings") {
        // 如果窗口已存在，检查是否最小化并恢复
        let is_minimized = settings_window.is_minimized().unwrap_or(false);

        if is_minimized {
            // 如果窗口被最小化，先取消最小化
            settings_window
                .unminimize()
                .map_err(|e| format!("取消最小化设置窗口失败: {}", e))?;
        }

        // 显示并聚焦窗口
        settings_window
            .show()
            .map_err(|e| format!("显示设置窗口失败: {}", e))?;
        settings_window
            .set_focus()
            .map_err(|e| format!("聚焦设置窗口失败: {}", e))?;
    } else {
        // 如果窗口不存在，创建新窗口
        let settings_window = tauri::WebviewWindowBuilder::new(
            &app,
            "settings",
            tauri::WebviewUrl::App("settings.html".into()),
        )
        .title("设置 - 快速剪贴板")
        .inner_size(900.0, 650.0)
        .min_inner_size(800.0, 600.0)
        .center()
        .resizable(true)
        .maximizable(true)
        .decorations(false) // 去除标题栏
        .build()
        .map_err(|e| format!("创建设置窗口失败: {}", e))?;

        // 设置窗口圆角（Windows 11）
        #[cfg(windows)]
        {
            if let Err(e) = crate::window_effects::set_window_rounded(&settings_window) {
                println!("设置设置窗口圆角失败: {}", e);
            }
        }

        settings_window
            .show()
            .map_err(|e| format!("显示设置窗口失败: {}", e))?;

        // 设置窗口关闭事件处理
        let app_handle = app.clone();
        settings_window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // 当设置窗口关闭时，隐藏主窗口（如果它是自动显示的）
                if let Some(main_window) = app_handle.get_webview_window("main") {
                    let _ = crate::window_management::hide_main_window_if_auto_shown(&main_window);
                }
            }
        });
    }

    Ok(())
}

// =================== 文本编辑窗口命令 ===================

// 打开文本编辑窗口
#[tauri::command]
pub async fn open_text_editor_window(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    // 检查文本编辑窗口是否已经存在
    if let Some(editor_window) = app.get_webview_window("text-editor") {
        // 如果窗口已存在，显示并聚焦
        editor_window
            .show()
            .map_err(|e| format!("显示文本编辑窗口失败: {}", e))?;
        editor_window
            .set_focus()
            .map_err(|e| format!("聚焦文本编辑窗口失败: {}", e))?;
    } else {
        // 如果窗口不存在，创建新窗口
        let editor_window = tauri::WebviewWindowBuilder::new(
            &app,
            "text-editor",
            tauri::WebviewUrl::App("textEditor.html".into()),
        )
        .title("文本编辑器 - 快速剪贴板")
        .inner_size(800.0, 600.0)
        .min_inner_size(400.0, 300.0)
        .center()
        .resizable(true)
        .maximizable(true)
        .decorations(false) // 去除标题栏
        .build()
        .map_err(|e| format!("创建文本编辑窗口失败: {}", e))?;

        // 设置窗口圆角（Windows 11）
        #[cfg(windows)]
        {
            if let Err(e) = crate::window_effects::set_window_rounded(&editor_window) {
                println!("设置文本编辑窗口圆角失败: {}", e);
            }
        }

        editor_window
            .show()
            .map_err(|e| format!("显示文本编辑窗口失败: {}", e))?;

        // 设置窗口关闭事件处理
        let app_handle = app.clone();
        editor_window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // 当编辑窗口关闭时，可以执行一些清理操作
                println!("文本编辑窗口已关闭");
            }
        });

        // 在开发模式下打开开发者工具
        #[cfg(debug_assertions)]
        {
            editor_window.open_devtools();
        }
    }

    Ok(())
}

// 获取设置
#[tauri::command]
pub fn get_settings() -> Result<serde_json::Value, String> {
    let settings = crate::settings::get_global_settings();
    Ok(settings.to_json())
}

#[tauri::command]
pub fn reload_settings() -> Result<serde_json::Value, String> {
    // 强制从文件重新加载设置
    let fresh_settings = crate::settings::AppSettings::load();

    // 更新全局设置
    if let Err(e) = crate::settings::update_global_settings(fresh_settings.clone()) {
        println!("更新全局设置失败: {}", e);
    }

    Ok(fresh_settings.to_json())
}

// 保存设置
#[tauri::command]
pub fn save_settings(settings: serde_json::Value) -> Result<(), String> {
    // println!("保存设置: {}", settings);

    // 更新全局设置
    crate::settings::update_global_settings_from_json(&settings)?;

    // 获取更新后的设置
    let app_settings = crate::settings::get_global_settings();

    // 应用各种设置
    // 1. 历史记录数量限制
    crate::clipboard_history::set_history_limit(app_settings.history_limit as usize);

    // 2. 开机自启动
    if let Err(e) = set_startup_launch(app_settings.auto_start) {
        println!("设置开机自启动失败: {}", e);
    }

    // 3. 剪贴板监听设置
    crate::clipboard_history::set_monitoring_enabled(app_settings.clipboard_monitor);

    // 4. 忽略重复内容设置
    crate::clipboard_history::set_ignore_duplicates(app_settings.ignore_duplicates);

    // 5. 保存图片设置
    crate::clipboard_history::set_save_images(app_settings.save_images);

    // 6. 数字快捷键设置
    #[cfg(windows)]
    crate::global_state::set_number_shortcuts_enabled(app_settings.number_shortcuts);

    // 7. 预览窗口快捷键设置
    #[cfg(windows)]
    crate::global_state::update_preview_shortcut_config(&app_settings.preview_shortcut);

    // 6. 更新音效设置
    let sound_settings = crate::sound_manager::SoundSettings {
        enabled: app_settings.sound_enabled,
        volume: (app_settings.sound_volume / 100.0) as f32, // 转换为0.0-1.0范围
        copy_sound_path: app_settings.copy_sound_path,
        paste_sound_path: app_settings.paste_sound_path,
        preset: app_settings.sound_preset,
    };
    crate::sound_manager::update_sound_settings(sound_settings);

    // 7. 截屏设置应用
    // 重新获取最新的设置以确保显示正确的值
    let updated_settings = crate::settings::get_global_settings();
    println!(
        "截屏设置已更新: 启用={}, 快捷键={}, 质量={}",
        updated_settings.screenshot_enabled,
        updated_settings.screenshot_shortcut,
        updated_settings.screenshot_quality
    );

    println!("设置保存成功");
    Ok(())
}

// 调试日志
#[tauri::command]
pub fn log_debug(message: String) {
    println!("前端调试: {}", message);
}

// 浏览音效文件
#[tauri::command]
pub async fn browse_sound_file() -> Result<Option<String>, String> {
    let dialog = rfd::AsyncFileDialog::new()
        .add_filter("音频文件", &["wav", "mp3", "ogg", "flac", "m4a", "aac"])
        .set_title("选择音效文件");

    if let Some(file) = dialog.pick_file().await {
        Ok(Some(file.path().to_string_lossy().to_string()))
    } else {
        Ok(None)
    }
}

// 测试音效（异步版本）
#[tauri::command]
pub async fn test_sound(sound_path: String, volume: f32) -> Result<(), String> {
    let volume_normalized = volume / 100.0; // 将0-100转换为0.0-1.0

    // 在后台线程中播放音效，避免阻塞前端
    let sound_path_clone = sound_path.clone();
    tokio::task::spawn_blocking(move || {
        let effective_path = if sound_path_clone.is_empty() {
            // 使用默认测试音效文件
            "sounds/copy.mp3".to_string() // 测试时使用复制音效
        } else {
            sound_path_clone
        };

        // 播放音效文件
        if let Err(e) =
            crate::sound_manager::SoundManager::play_sound_sync(&effective_path, volume_normalized)
        {
            eprintln!("测试音效失败: {}", e);
            // 如果文件播放失败，回退到代码生成的音效
            if let Err(e2) =
                crate::sound_manager::SoundManager::play_beep(700.0, 200, volume_normalized)
            {
                eprintln!("测试默认音效也失败: {}", e2);
            }
        }
    })
    .await
    .map_err(|e| format!("音效测试任务失败: {}", e))?;

    Ok(())
}

// 播放粘贴音效（供键盘钩子调用）
#[tauri::command]
pub fn play_paste_sound() -> Result<(), String> {
    crate::sound_manager::play_paste_sound();
    Ok(())
}

// 播放滚动音效（供预览窗口调用）
#[tauri::command]
pub fn play_scroll_sound() -> Result<(), String> {
    crate::sound_manager::play_scroll_sound();
    Ok(())
}

// 清理音效缓存
#[tauri::command]
pub fn clear_sound_cache() -> Result<(), String> {
    crate::sound_manager::clear_sound_cache()
}

// 获取当前活跃音效播放数量
#[tauri::command]
pub fn get_active_sound_count() -> usize {
    crate::sound_manager::get_active_sound_count()
}

// 从剪贴板历史添加到分组
#[tauri::command]
pub fn add_clipboard_to_group(index: usize, group_id: String) -> Result<QuickText, String> {
    let history = clipboard_history::CLIPBOARD_HISTORY.lock().unwrap();

    if index >= history.len() {
        return Err(format!("索引 {} 超出历史范围", index));
    }

    let content = history[index].clone();
    drop(history); // 释放锁

    // 处理内容，如果是图片则创建副本
    let final_content = if content.starts_with("image:") {
        // 提取图片ID
        let image_id = content.strip_prefix("image:").unwrap_or("");
        if !image_id.is_empty() {
            // 创建图片副本
            match get_image_manager() {
                Ok(image_manager) => {
                    match image_manager.lock() {
                        Ok(manager) => {
                            match manager.copy_image(image_id) {
                                Ok(new_image_info) => {
                                    format!("image:{}", new_image_info.id)
                                }
                                Err(e) => {
                                    println!("复制图片失败: {}, 使用原始引用", e);
                                    content // 如果复制失败，使用原始引用
                                }
                            }
                        }
                        Err(e) => {
                            println!("获取图片管理器锁失败: {}, 使用原始引用", e);
                            content
                        }
                    }
                }
                Err(e) => {
                    println!("获取图片管理器失败: {}, 使用原始引用", e);
                    content
                }
            }
        } else {
            content
        }
    } else {
        content.clone()
    };

    // 生成标题：根据内容类型生成合适的标题
    let title = if final_content.starts_with("data:image/") || final_content.starts_with("image:") {
        // 图片内容使用固定标题
        "图片".to_string()
    } else if final_content.starts_with("files:") {
        // 文件内容解析文件名作为标题
        generate_files_title(&final_content)
    } else {
        // 安全地截取字符串，避免在UTF-8字符中间截断
        let chars: Vec<char> = final_content.chars().collect();
        if chars.len() > 30 {
            format!("{}...", chars.iter().take(30).collect::<String>())
        } else {
            final_content.clone()
        }
    };

    // 添加到指定分组的常用文本
    quick_texts::add_quick_text_with_group(title, final_content, group_id)
}

// 设置主窗口为超级置顶（确保在开始菜单之上）
#[tauri::command]
pub fn set_super_topmost(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        #[cfg(windows)]
        {
            crate::window_management::set_super_topmost_window(&window)
                .map_err(|e| format!("设置超级置顶失败: {}", e))?;
            println!("主窗口已设置为超级置顶");
        }
        Ok(())
    } else {
        Err("找不到主窗口".to_string())
    }
}

// 获取音效播放状态
#[tauri::command]
pub fn get_sound_status() -> Result<serde_json::Value, String> {
    let active_count = crate::sound_manager::get_active_sound_count();
    Ok(serde_json::json!({
        "active_sounds": active_count,
        "max_concurrent": 3
    }))
}

// 获取图片数据URL
#[tauri::command]
pub fn get_image_data_url(image_id: String) -> Result<String, String> {
    let image_manager = get_image_manager()?;
    let manager = image_manager
        .lock()
        .map_err(|e| format!("获取图片管理器锁失败: {}", e))?;
    manager.get_image_data_url(&image_id)
}

// 获取图片缩略图数据URL
#[tauri::command]
pub fn get_image_thumbnail_url(image_id: String) -> Result<String, String> {
    let image_manager = get_image_manager()?;
    let manager = image_manager
        .lock()
        .map_err(|e| format!("获取图片管理器锁失败: {}", e))?;
    manager.get_thumbnail_data_url(&image_id)
}

// 设置预览窗口当前索引
#[tauri::command]
pub fn set_preview_index(index: usize) -> Result<(), String> {
    crate::preview_window::set_preview_index(index);
    Ok(())
}

// 取消预览（不粘贴直接隐藏）
#[tauri::command]
pub async fn cancel_preview() -> Result<(), String> {
    crate::preview_window::cancel_preview().await
}

// 删除剪贴板项目
#[tauri::command]
pub fn delete_clipboard_item(id: usize) -> Result<(), String> {
    clipboard_history::delete_item_by_id(id)
}

// 更新剪贴板项目内容
#[tauri::command]
pub fn update_clipboard_item(index: usize, content: String) -> Result<(), String> {
    clipboard_history::update_item_content(index, content)
}

// 清空剪贴板历史
#[tauri::command]
pub fn clear_clipboard_history() -> Result<(), String> {
    clipboard_history::clear_all()
}

// 发送剪贴板更新事件
#[tauri::command]
pub async fn emit_clipboard_updated(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;

    // 发送事件到主窗口
    if let Some(main_window) = app.get_webview_window("main") {
        let _ = main_window.emit("clipboard-changed", ());
    }

    // 发送事件到预览窗口
    if let Some(preview_window) = app.get_webview_window("preview") {
        let _ = preview_window.emit("clipboard-history-updated", ());
    }

    Ok(())
}

// 发送常用文本更新事件
#[tauri::command]
pub async fn emit_quick_texts_updated(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;

    // 发送事件到主窗口
    if let Some(main_window) = app.get_webview_window("main") {
        let _ = main_window.emit("refreshQuickTexts", ());
    }

    // 发送事件到预览窗口
    if let Some(preview_window) = app.get_webview_window("preview") {
        let _ = preview_window.emit("quick-texts-updated", ());
    }

    Ok(())
}

// 通知预览窗口标签切换
#[tauri::command]
pub fn notify_preview_tab_change(tab: String, group_id: String) -> Result<(), String> {
    crate::preview_window::update_preview_source(tab, group_id)
}

// 获取主窗口当前状态
#[tauri::command]
pub fn get_main_window_state() -> Result<serde_json::Value, String> {
    crate::preview_window::get_main_window_state()
}

// 更新主题设置
#[tauri::command]
pub fn update_theme_setting(theme: String) -> Result<(), String> {
    let mut settings = crate::settings::get_global_settings();
    settings.theme = theme;
    crate::settings::update_global_settings(settings)?;
    Ok(())
}

// 获取应用版本信息
#[tauri::command]
pub fn get_app_version() -> Result<serde_json::Value, String> {
    let version = env!("CARGO_PKG_VERSION");
    let name = env!("CARGO_PKG_NAME");
    let description = env!("CARGO_PKG_DESCRIPTION");
    let authors = env!("CARGO_PKG_AUTHORS");

    let version_info = serde_json::json!({
        "version": version,
        "name": name,
        "description": description,
        "authors": authors
    });

    Ok(version_info)
}

// =================== 管理员权限相关命令 ===================

// 获取管理员权限状态
#[tauri::command]
pub fn get_admin_status() -> Result<admin_privileges::AdminStatus, String> {
    Ok(admin_privileges::get_admin_status())
}

// 以管理员权限重启应用
#[tauri::command]
pub fn restart_as_admin() -> Result<(), String> {
    admin_privileges::restart_as_admin()
}

// 检查后端是否初始化完成
#[tauri::command]
pub fn is_backend_initialized() -> bool {
    crate::BACKEND_INITIALIZED.load(std::sync::atomic::Ordering::Relaxed)
}

// =================== 系统通知相关命令 ===================

// 发送系统通知
#[tauri::command]
pub fn send_system_notification(title: String, body: String) -> Result<(), String> {
    println!("发送系统通知: {} - {}", title, body);
    Ok(())
}

// 发送启动通知
#[tauri::command]
pub fn send_startup_notification(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_notification::NotificationExt;

    let admin_status = admin_privileges::get_admin_status();
    let status_text = if admin_status.is_admin {
        "（管理员模式）"
    } else {
        ""
    };

    // 获取当前设置的快捷键
    let app_settings = crate::settings::get_global_settings();
    let shortcut_key = if app_settings.toggle_shortcut.is_empty() {
        "Win+V".to_string()
    } else {
        app_settings.toggle_shortcut.clone()
    };

    let notification_body = format!(
        "QuickClipboard 已启动{}\n按 {} 打开剪贴板",
        status_text, shortcut_key
    );

    match app
        .notification()
        .builder()
        .title("QuickClipboard")
        .body(&notification_body)
        .show()
    {
        Ok(_) => {
            println!("启动通知发送成功");
            Ok(())
        }
        Err(e) => {
            println!("发送启动通知失败: {}", e);
            Err(format!("发送通知失败: {}", e))
        }
    }
}

// =================== AI翻译相关命令 ===================

/// 测试AI翻译配置
#[tauri::command]
pub async fn test_ai_translation() -> Result<String, String> {
    let settings = crate::settings::get_global_settings();

    // 检查配置是否有效
    if !crate::ai_translator::is_translation_config_valid(&settings) {
        return Err("AI翻译配置不完整，请检查API密钥、模型和目标语言设置".to_string());
    }

    // 创建翻译配置
    let config = crate::ai_translator::config_from_settings(&settings);

    // 创建翻译器
    let translator = match crate::ai_translator::AITranslator::new(config) {
        Ok(t) => t,
        Err(e) => return Err(format!("创建翻译器失败: {}", e)),
    };

    // 测试翻译
    let test_text = "Hello, this is a test message for AI translation.";

    match translator.translate_stream(test_text).await {
        Ok(mut receiver) => {
            let mut result = String::new();

            // 收集流式响应
            while let Some(translation_result) = receiver.recv().await {
                match translation_result {
                    crate::ai_translator::TranslationResult::Chunk(chunk) => {
                        result.push_str(&chunk);
                    }
                    crate::ai_translator::TranslationResult::Complete => {
                        break;
                    }
                    crate::ai_translator::TranslationResult::Error(e) => {
                        return Err(format!("翻译失败: {}", e));
                    }
                }
            }

            if result.is_empty() {
                Err("翻译结果为空".to_string())
            } else {
                Ok(format!("测试成功！翻译结果：{}", result))
            }
        }
        Err(e) => Err(format!("启动翻译失败: {}", e)),
    }
}

/// 取消正在进行的翻译
#[tauri::command]
pub fn cancel_translation() -> Result<(), String> {
    TRANSLATION_CANCELLED.store(true, Ordering::SeqCst);
    println!("翻译已被用户取消");
    Ok(())
}

/// 启用AI翻译取消快捷键
#[tauri::command]
pub fn enable_ai_translation_cancel_shortcut() -> Result<(), String> {
    #[cfg(windows)]
    crate::global_state::enable_ai_translation_cancel();
    Ok(())
}

/// 禁用AI翻译取消快捷键
#[tauri::command]
pub fn disable_ai_translation_cancel_shortcut() -> Result<(), String> {
    #[cfg(windows)]
    crate::global_state::disable_ai_translation_cancel();
    Ok(())
}

/// 翻译文本并直接粘贴（非流式）
#[tauri::command]
pub async fn translate_and_paste_text(text: String) -> Result<(), String> {
    // 重置取消状态
    TRANSLATION_CANCELLED.store(false, Ordering::SeqCst);

    // 启用AI翻译取消快捷键
    #[cfg(windows)]
    crate::global_state::enable_ai_translation_cancel();

    // 确保在函数结束时禁用快捷键
    let _guard = TranslationGuard;

    let settings = crate::settings::get_global_settings();

    // 检查翻译是否启用
    if !settings.ai_translation_enabled {
        return Err("AI翻译功能未启用".to_string());
    }

    // 检查配置是否有效
    if !crate::ai_translator::is_translation_config_valid(&settings) {
        return Err("AI翻译配置不完整".to_string());
    }

    // 预处理输入文本
    let processed_text = preprocess_translation_text(&text)?;

    // 创建翻译配置
    let translation_config = crate::ai_translator::config_from_settings(&settings);

    // 创建翻译器
    let translator = match crate::ai_translator::AITranslator::new(translation_config) {
        Ok(t) => t,
        Err(e) => return Err(format!("创建翻译器失败: {}", e)),
    };

    // 开始翻译（非流式）
    match translator.translate(&processed_text).await {
        Ok(translated_text) => {
            // 检查是否被取消
            if TRANSLATION_CANCELLED.load(Ordering::SeqCst) {
                println!("翻译在粘贴前被用户取消");
                return Err("翻译已被取消".to_string());
            }

            println!("翻译完成，结果长度: {} 字符", translated_text.len());

            // 设置剪贴板内容并粘贴
            crate::clipboard_monitor::set_pasting_state(true);

            // 使用现有的剪贴板设置功能
            set_clipboard_content_no_history(translated_text)?;

            // 执行粘贴操作
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(10));
                #[cfg(windows)]
                windows_paste();

                // 播放粘贴音效
                crate::sound_manager::play_paste_sound();

                // 粘贴完成后重置粘贴状态
                thread::sleep(Duration::from_millis(500));
                crate::clipboard_monitor::set_pasting_state(false);
            });

            Ok(())
        }
        Err(e) => Err(format!("翻译失败: {}", e)),
    }
}

/// 翻译文本并流式输入（改进版本）
#[tauri::command]
pub async fn translate_and_input_text(text: String) -> Result<(), String> {
    // 重置取消状态
    TRANSLATION_CANCELLED.store(false, Ordering::SeqCst);

    // 启用AI翻译取消快捷键
    #[cfg(windows)]
    crate::global_state::enable_ai_translation_cancel();

    // 确保在函数结束时禁用快捷键
    let _guard = TranslationGuard;

    let settings = crate::settings::get_global_settings();

    // 检查翻译是否启用
    if !settings.ai_translation_enabled {
        return Err("AI翻译功能未启用".to_string());
    }

    // 检查配置是否有效
    if !crate::ai_translator::is_translation_config_valid(&settings) {
        return Err("AI翻译配置不完整".to_string());
    }

    // 预处理输入文本
    let processed_text = preprocess_translation_text(&text)?;

    // 创建翻译配置
    let translation_config = crate::ai_translator::config_from_settings(&settings);
    let input_config = crate::text_input_simulator::config_from_settings(&settings);

    // 创建翻译器
    let translator = match crate::ai_translator::AITranslator::new(translation_config) {
        Ok(t) => t,
        Err(e) => return Err(format!("创建翻译器失败: {}", e)),
    };

    // 更新输入模拟器配置
    crate::text_input_simulator::update_global_input_simulator_config(input_config);

    // 开始翻译
    match translator.translate_stream(&processed_text).await {
        Ok(mut receiver) => {
            let mut accumulated_text = String::new();
            let mut chunk_count = 0;

            // 处理流式响应并实时输入
            while let Some(translation_result) = receiver.recv().await {
                // 检查是否被取消
                if TRANSLATION_CANCELLED.load(Ordering::SeqCst) {
                    println!("翻译被用户取消");
                    return Err("翻译已被取消".to_string());
                }

                match translation_result {
                    crate::ai_translator::TranslationResult::Chunk(chunk) => {
                        // 再次检查是否被取消（在输入前）
                        if TRANSLATION_CANCELLED.load(Ordering::SeqCst) {
                            println!("翻译在输入前被用户取消");
                            return Err("翻译已被取消".to_string());
                        }

                        // 详细的AI返回内容调试
                        let newline_count = chunk.matches('\n').count();
                        let has_digits = chunk.chars().any(|c| c.is_ascii_digit());
                        let has_colon = chunk.contains(':') || chunk.contains('：');

                        if newline_count > 0 || has_digits || has_colon || chunk.len() > 20 {
                            println!(
                                "AI片段{}: 长度={}, 换行符={}, 数字={}, 冒号={}, 内容: {:?}",
                                chunk_count + 1,
                                chunk.len(),
                                newline_count,
                                has_digits,
                                has_colon,
                                chunk
                            );
                        }

                        // 累积文本用于错误恢复
                        accumulated_text.push_str(&chunk);
                        chunk_count += 1;

                        // 使用改进的智能输入方法
                        match crate::text_input_simulator::simulate_text_chunk_input_smart(&chunk)
                            .await
                        {
                            Ok(()) => {
                                // 成功时不输出信息，减少日志噪音
                            }
                            Err(e) => {
                                println!("输入失败: {}", e);

                                // 尝试使用降级输入方法
                                match crate::text_input_simulator::simulate_text_chunk_input_precise(&chunk).await {
                                    Ok(()) => {
                                        println!("降级输入成功");
                                    }
                                    Err(fallback_error) => {
                                        println!("降级输入也失败: {}", fallback_error);
                                        // 继续处理下一个片段，而不是完全失败
                                    }
                                }
                            }
                        }
                    }
                    crate::ai_translator::TranslationResult::Complete => {
                        println!(
                            "翻译完成，总共处理 {} 个片段，累积长度: {}",
                            chunk_count,
                            accumulated_text.len()
                        );
                        break;
                    }
                    crate::ai_translator::TranslationResult::Error(e) => {
                        return Err(format!("翻译失败: {}", e));
                    }
                }
            }

            Ok(())
        }
        Err(e) => Err(format!("启动翻译失败: {}", e)),
    }
}

/// 翻译守护结构，确保在函数结束时清理资源
struct TranslationGuard;

impl Drop for TranslationGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        crate::global_state::disable_ai_translation_cancel();
    }
}

/// 预处理翻译文本
fn preprocess_translation_text(text: &str) -> Result<String, String> {
    // 检查文本长度
    if text.is_empty() {
        return Err("输入文本为空".to_string());
    }

    if text.len() > 50_000 {
        return Err("输入文本过长，超过50KB限制".to_string());
    }

    // 规范化文本格式
    let normalized = text
        .replace("\r\n", "\n") // 统一换行符
        .replace('\r', "\n"); // 处理Mac格式换行符

    // 移除过多的连续空行（保留最多2个连续换行）
    let cleaned = regex::Regex::new(r"\n{3,}")
        .unwrap()
        .replace_all(&normalized, "\n\n")
        .to_string();

    Ok(cleaned)
}

/// 智能翻译文本（根据设置选择流式输入或直接粘贴）
#[tauri::command]
pub async fn translate_text_smart(text: String) -> Result<(), String> {
    let settings = crate::settings::get_global_settings();

    // 根据输出模式设置选择翻译方式
    match settings.ai_output_mode.as_str() {
        "paste" => {
            println!("使用直接粘贴模式进行翻译");
            translate_and_paste_text(text).await
        }
        "stream" | _ => {
            println!("使用流式输入模式进行翻译");
            translate_and_input_text(text).await
        }
    }
}

/// 复制时翻译并直接输入到目标位置
#[tauri::command]
pub async fn translate_and_input_on_copy(text: String) -> Result<(), String> {
    // 重置取消状态
    TRANSLATION_CANCELLED.store(false, Ordering::SeqCst);

    // 启用AI翻译取消快捷键
    #[cfg(windows)]
    crate::global_state::enable_ai_translation_cancel();

    // 确保在函数结束时禁用快捷键
    let _guard = TranslationGuard;

    let settings = crate::settings::get_global_settings();

    // 检查翻译是否启用
    if !settings.ai_translation_enabled {
        return Err("AI翻译功能未启用".to_string());
    }

    // 检查配置是否有效
    if !crate::ai_translator::is_translation_config_valid(&settings) {
        return Err("AI翻译配置不完整".to_string());
    }

    // 预处理输入文本
    let processed_text = preprocess_translation_text(&text)?;

    // 创建翻译配置和输入配置
    let translation_config = crate::ai_translator::config_from_settings(&settings);
    let input_config = crate::text_input_simulator::config_from_settings(&settings);

    // 创建翻译器
    let translator = match crate::ai_translator::AITranslator::new(translation_config) {
        Ok(t) => t,
        Err(e) => return Err(format!("创建翻译器失败: {}", e)),
    };

    // 更新输入模拟器配置
    crate::text_input_simulator::update_global_input_simulator_config(input_config);

    println!("开始复制时翻译，原文长度: {} 字符", processed_text.len());

    // 根据输出模式选择翻译方式
    match settings.ai_output_mode.as_str() {
        "paste" => {
            // 直接粘贴模式：翻译后设置剪贴板并粘贴
            println!("复制时翻译使用直接粘贴模式");
            match translator.translate(&processed_text).await {
                Ok(translated_text) => {
                    // 检查是否被取消
                    if TRANSLATION_CANCELLED.load(Ordering::SeqCst) {
                        println!("翻译在粘贴前被用户取消");
                        return Err("翻译已被取消".to_string());
                    }

                    println!("复制时翻译完成，结果长度: {} 字符", translated_text.len());

                    // 设置粘贴状态，防止触发新的复制检测
                    crate::clipboard_monitor::set_pasting_state(true);

                    // 设置剪贴板内容并粘贴
                    set_clipboard_content_no_history(translated_text)?;

                    // 执行粘贴操作
                    thread::spawn(move || {
                        thread::sleep(Duration::from_millis(10));
                        #[cfg(windows)]
                        windows_paste();

                        // 播放粘贴音效
                        crate::sound_manager::play_paste_sound();

                        // 粘贴完成后重置粘贴状态
                        thread::sleep(Duration::from_millis(500));
                        crate::clipboard_monitor::set_pasting_state(false);
                    });

                    Ok(())
                }
                Err(e) => Err(format!("复制时翻译失败: {}", e)),
            }
        }
        "stream" | _ => {
            // 流式输入模式：翻译后直接输入到目标位置
            println!("复制时翻译使用流式输入模式");
            match translator.translate_stream(&processed_text).await {
                Ok(mut receiver) => {
                    let mut accumulated_text = String::new();
                    let mut chunk_count = 0;

                    // 处理流式翻译结果
                    loop {
                        // 检查是否被取消
                        if TRANSLATION_CANCELLED.load(Ordering::SeqCst) {
                            println!("复制时翻译被用户取消");
                            return Err("翻译已被取消".to_string());
                        }

                        match receiver.recv().await {
                            Some(result) => match result {
                                crate::ai_translator::TranslationResult::Chunk(chunk) => {
                                    // 累积文本用于错误恢复
                                    accumulated_text.push_str(&chunk);
                                    chunk_count += 1;

                                    // 使用改进的智能输入方法
                                    match crate::text_input_simulator::simulate_text_chunk_input_smart(&chunk).await {
                                        Ok(()) => {
                                            // 成功时不输出信息，减少日志噪音
                                        }
                                        Err(e) => {
                                            println!("输入失败: {}", e);

                                            // 尝试使用降级输入方法
                                            match crate::text_input_simulator::simulate_text_chunk_input_precise(&chunk).await {
                                                Ok(()) => {
                                                    println!("降级输入成功");
                                                }
                                                Err(fallback_error) => {
                                                    println!("降级输入也失败: {}", fallback_error);
                                                    // 继续处理下一个片段，而不是完全失败
                                                }
                                            }
                                        }
                                    }
                                }
                                crate::ai_translator::TranslationResult::Complete => {
                                    println!(
                                        "复制时翻译完成，总共处理 {} 个片段，累积长度: {}",
                                        chunk_count,
                                        accumulated_text.len()
                                    );
                                    break;
                                }
                                crate::ai_translator::TranslationResult::Error(e) => {
                                    return Err(format!("复制时翻译失败: {}", e));
                                }
                            },
                            None => {
                                println!("翻译流意外结束");
                                break;
                            }
                        }
                    }

                    Ok(())
                }
                Err(e) => Err(format!("启动复制时翻译失败: {}", e)),
            }
        }
    }
}

/// 检查当前是否处于粘贴状态
#[tauri::command]
pub fn is_currently_pasting() -> bool {
    crate::clipboard_monitor::is_currently_pasting()
}

/// 检查AI翻译配置是否有效
#[tauri::command]
pub fn check_ai_translation_config() -> Result<bool, String> {
    let settings = crate::settings::get_global_settings();
    Ok(crate::ai_translator::is_translation_config_valid(&settings))
}

// =================== 文件处理命令 ===================

#[tauri::command]
pub async fn copy_files_to_directory(
    files: Vec<String>,
    target_dir: String,
) -> Result<Vec<String>, String> {
    crate::file_handler::copy_files_to_target(&files, &target_dir)
}

#[tauri::command]
pub async fn get_file_info(path: String) -> Result<crate::file_handler::FileInfo, String> {
    crate::file_handler::get_file_info(&path)
}

#[tauri::command]
pub async fn get_clipboard_files() -> Result<Vec<String>, String> {
    crate::file_handler::get_clipboard_files()
}

#[tauri::command]
pub async fn set_clipboard_files(files: Vec<String>) -> Result<(), String> {
    crate::file_handler::set_clipboard_files(&files)
}

/// 获取可用的AI模型列表
#[tauri::command]
pub async fn get_available_ai_models() -> Result<Vec<String>, String> {
    let settings = crate::settings::get_global_settings();
    let ai_config = crate::ai_config::create_ai_config_from_settings(&settings);

    if !ai_config.is_valid() {
        return Err("AI配置无效，请检查API密钥等设置".to_string());
    }

    let config_manager = crate::ai_config::AIConfigManager::new(ai_config)
        .map_err(|e| format!("创建AI配置管理器失败: {}", e))?;

    config_manager
        .get_available_models()
        .await
        .map_err(|e| format!("获取模型列表失败: {}", e))
}

/// 测试AI配置
#[tauri::command]
pub async fn test_ai_config() -> Result<bool, String> {
    let settings = crate::settings::get_global_settings();
    let ai_config = crate::ai_config::create_ai_config_from_settings(&settings);

    if !ai_config.is_valid() {
        return Err("AI配置无效".to_string());
    }

    let config_manager = crate::ai_config::AIConfigManager::new(ai_config)
        .map_err(|e| format!("创建AI配置管理器失败: {}", e))?;

    config_manager
        .test_config()
        .await
        .map_err(|e| format!("AI配置测试失败: {}", e))?;

    Ok(true)
}

// 打开文件位置
#[tauri::command]
pub async fn open_file_location(file_path: String) -> Result<(), String> {
    use std::process::Command;

    #[cfg(windows)]
    {
        // Windows: 使用 explorer 打开文件位置并选中文件
        let result = Command::new("explorer")
            .args(&["/select,", &file_path])
            .spawn();

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("打开文件位置失败: {}", e)),
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: 使用 Finder 打开文件位置
        let result = Command::new("open").args(&["-R", &file_path]).spawn();

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("打开文件位置失败: {}", e)),
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: 尝试使用文件管理器打开
        let result = Command::new("xdg-open")
            .arg(
                std::path::Path::new(&file_path)
                    .parent()
                    .unwrap_or(std::path::Path::new("/")),
            )
            .spawn();

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("打开文件位置失败: {}", e)),
        }
    }
}

// 统一粘贴参数
#[derive(serde::Deserialize)]
pub struct PasteContentParams {
    pub content: String,
    pub quick_text_id: Option<String>, // 如果是常用文本，提供ID
    pub one_time: Option<bool>,        // 是否一次性粘贴
}

// 统一粘贴命令 - 自动识别内容类型并执行相应的粘贴操作
#[tauri::command]
pub async fn paste_content(
    params: PasteContentParams,
    window: WebviewWindow,
) -> Result<(), String> {
    println!("统一粘贴命令被调用，内容长度: {}", params.content.len());

    // 判断内容类型并执行相应的粘贴操作
    if params.content.starts_with("files:") {
        // 文件类型粘贴
        paste_files_internal(params.content, window).await
    } else if params.content.starts_with("data:image/") || params.content.starts_with("image:") {
        // 图片类型粘贴
        paste_image_internal(params.content, window).await
    } else {
        // 文本类型粘贴
        paste_text_internal(params.content, window).await
    }?;

    // 处理一次性粘贴逻辑
    if let (Some(quick_text_id), Some(true)) = (params.quick_text_id, params.one_time) {
        // 删除一次性常用文本
        if let Err(e) = quick_texts::delete_quick_text(quick_text_id) {
            eprintln!("删除一次性常用文本失败: {}", e);
        }
    }

    Ok(())
}

// 内部文件粘贴函数
async fn paste_files_internal(files_data: String, window: WebviewWindow) -> Result<(), String> {
    // 解析文件数据
    if !files_data.starts_with("files:") {
        return Err("无效的文件数据格式".to_string());
    }

    let files_json = &files_data[6..]; // 去掉 "files:" 前缀
    let files_data: serde_json::Value =
        serde_json::from_str(files_json).map_err(|e| format!("解析文件数据失败: {}", e))?;

    let files = files_data["files"].as_array().ok_or("文件数据格式错误")?;

    let file_paths: Vec<String> = files
        .iter()
        .filter_map(|file| file["path"].as_str())
        .map(|path| path.to_string())
        .collect();

    if file_paths.is_empty() {
        return Err("没有找到有效的文件路径".to_string());
    }

    // 设置剪贴板文件
    crate::file_handler::set_clipboard_files(&file_paths)?;

    // 执行粘贴操作
    if !crate::paste_utils::windows_paste() {
        return Err("粘贴操作失败".to_string());
    }

    // 播放粘贴音效
    crate::sound_manager::play_paste_sound();

    // 处理窗口显示/隐藏
    let is_pinned = crate::window_management::get_window_pinned();
    if !is_pinned {
        if let Err(e) = window.hide() {
            eprintln!("隐藏窗口失败: {}", e);
        }
    }

    Ok(())
}

// 内部图片粘贴函数
async fn paste_image_internal(image_content: String, window: WebviewWindow) -> Result<(), String> {
    // 设置粘贴状态，防止移动历史记录位置
    crate::clipboard_monitor::set_pasting_state(true);

    // 处理图片内容到剪贴板
    if image_content.starts_with("image:") {
        // 新格式：image:id，需要通过图片管理器获取完整数据
        let image_id = &image_content[6..];
        let image_manager = crate::image_manager::get_image_manager().map_err(|e| {
            crate::clipboard_monitor::set_pasting_state(false);
            format!("获取图片管理器失败: {}", e)
        })?;
        let image_data = image_manager
            .lock()
            .unwrap()
            .get_image_data_url(image_id)
            .map_err(|e| {
                crate::clipboard_monitor::set_pasting_state(false);
                format!("获取图片数据失败: {}", e)
            })?;

        // 将图片数据设置到剪贴板（不添加到历史记录，避免重复）
        if let Err(e) = crate::clipboard_content::set_clipboard_content_no_history(image_data) {
            crate::clipboard_monitor::set_pasting_state(false);
            return Err(e);
        }
    } else if image_content.starts_with("data:image/") {
        // 旧格式：完整的data URL
        if let Err(e) = crate::clipboard_content::set_clipboard_content_no_history(image_content) {
            crate::clipboard_monitor::set_pasting_state(false);
            return Err(e);
        }
    } else {
        crate::clipboard_monitor::set_pasting_state(false);
        return Err("不支持的图片格式".to_string());
    }

    // 执行粘贴操作
    if !crate::paste_utils::windows_paste() {
        crate::clipboard_monitor::set_pasting_state(false);
        return Err("粘贴操作失败".to_string());
    }

    // 播放粘贴音效
    crate::sound_manager::play_paste_sound();

    // 处理窗口显示/隐藏
    let is_pinned = crate::window_management::get_window_pinned();
    if !is_pinned {
        if let Err(e) = window.hide() {
            eprintln!("隐藏窗口失败: {}", e);
        }
    }

    // 延迟重置粘贴状态，确保剪贴板监听器能正确识别
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(500));
        crate::clipboard_monitor::set_pasting_state(false);
    });

    Ok(())
}

// 内部文本粘贴函数
async fn paste_text_internal(text_content: String, window: WebviewWindow) -> Result<(), String> {
    // 设置粘贴状态，防止移动历史记录位置
    crate::clipboard_monitor::set_pasting_state(true);

    // 将文本设置到剪贴板（不添加到历史记录，避免重复）
    if let Err(e) = crate::clipboard_content::set_clipboard_content_no_history(text_content) {
        crate::clipboard_monitor::set_pasting_state(false);
        return Err(e);
    }

    // 执行粘贴操作
    if !crate::paste_utils::windows_paste() {
        crate::clipboard_monitor::set_pasting_state(false);
        return Err("粘贴操作失败".to_string());
    }

    // 播放粘贴音效
    crate::sound_manager::play_paste_sound();

    // 处理窗口显示/隐藏
    let is_pinned = crate::window_management::get_window_pinned();
    if !is_pinned {
        if let Err(e) = window.hide() {
            eprintln!("隐藏窗口失败: {}", e);
        }
    }

    // 延迟重置粘贴状态，确保剪贴板监听器能正确识别
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_millis(500));
        crate::clipboard_monitor::set_pasting_state(false);
    });

    Ok(())
}

// 生成文件类型的标题
fn generate_files_title(files_content: &str) -> String {
    // 解析文件数据
    if let Some(files_json) = files_content.strip_prefix("files:") {
        if let Ok(files_data) = serde_json::from_str::<serde_json::Value>(files_json) {
            if let Some(files_array) = files_data["files"].as_array() {
                let file_count = files_array.len();

                if file_count == 0 {
                    return "空文件列表".to_string();
                } else if file_count == 1 {
                    // 单个文件，显示文件名
                    if let Some(file_name) = files_array[0]["name"].as_str() {
                        return file_name.to_string();
                    }
                } else {
                    // 多个文件，显示第一个文件名和数量
                    if let Some(first_file_name) = files_array[0]["name"].as_str() {
                        return format!("{} 等 {} 个文件", first_file_name, file_count);
                    } else {
                        return format!("{} 个文件", file_count);
                    }
                }
            }
        }
    }

    // 解析失败时的回退标题
    "文件".to_string()
}
