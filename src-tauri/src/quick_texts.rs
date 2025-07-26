use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

// 常用文本数据结构
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuickText {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default = "default_group_id")]
    pub group_id: String,
}

// 默认分组ID
fn default_group_id() -> String {
    "all".to_string()
}

// 持久化文件路径
pub static QUICK_TEXTS_FILE: Lazy<PathBuf> = Lazy::new(|| {
    let mut dir = dirs::data_local_dir().unwrap_or_else(|| std::env::temp_dir());
    dir.push("quickclipboard");
    let _ = std::fs::create_dir_all(&dir);
    dir.push("quick_texts.json");
    dir
});

// 使用Lazy和Mutex来存储常用文本
pub static QUICK_TEXTS: Lazy<std::sync::Mutex<HashMap<String, QuickText>>> =
    Lazy::new(|| std::sync::Mutex::new(HashMap::new()));

// 保存常用文本到文件
pub fn save_quick_texts(texts: &HashMap<String, QuickText>) {
    let vec: Vec<QuickText> = texts.values().cloned().collect();
    match serde_json::to_string_pretty(&vec) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&*QUICK_TEXTS_FILE, json) {
                println!("保存常用文本失败: {}", e);
            }
        }
        Err(e) => println!("序列化常用文本失败: {}", e),
    }
}

// 从文件加载常用文本
pub fn load_quick_texts() {
    if let Ok(content) = std::fs::read_to_string(&*QUICK_TEXTS_FILE) {
        if let Ok(list) = serde_json::from_str::<Vec<QuickText>>(&content) {
            let mut texts = QUICK_TEXTS.lock().unwrap();
            texts.clear();
            for text in list {
                texts.insert(text.id.clone(), text);
            }
        }
    }
}

// 获取所有常用文本
pub fn get_all_quick_texts() -> Vec<QuickText> {
    let texts = QUICK_TEXTS.lock().unwrap();
    let mut result: Vec<QuickText> = texts.values().cloned().collect();
    // 按更新时间倒序排列
    result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    result
}

// 按分组获取常用文本
pub fn get_quick_texts_by_group(group_id: &str) -> Vec<QuickText> {
    let texts = QUICK_TEXTS.lock().unwrap();
    let mut result: Vec<QuickText> = texts
        .values()
        .filter(|text| text.group_id == group_id)
        .cloned()
        .collect();
    // 按更新时间倒序排列
    result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    result
}

// 添加常用文本
pub fn add_quick_text(title: String, content: String) -> Result<QuickText, String> {
    add_quick_text_with_group(title, content, "all".to_string())
}

// 添加常用文本到指定分组
pub fn add_quick_text_with_group(
    title: String,
    content: String,
    group_id: String,
) -> Result<QuickText, String> {
    let now = chrono::Utc::now().timestamp();
    let quick_text = QuickText {
        id: Uuid::new_v4().to_string(),
        title,
        content,
        created_at: now,
        updated_at: now,
        group_id,
    };

    let mut texts = QUICK_TEXTS.lock().unwrap();
    texts.insert(quick_text.id.clone(), quick_text.clone());
    save_quick_texts(&texts);

    Ok(quick_text)
}

// 更新常用文本
pub fn update_quick_text(id: String, title: String, content: String) -> Result<QuickText, String> {
    update_quick_text_with_group(id, title, content, None)
}

// 更新常用文本（包含分组）
pub fn update_quick_text_with_group(
    id: String,
    title: String,
    content: String,
    group_id: Option<String>,
) -> Result<QuickText, String> {
    let mut texts = QUICK_TEXTS.lock().unwrap();

    if let Some(text) = texts.get_mut(&id) {
        text.title = title;
        text.content = content;
        // 总是更新分组ID，如果是None则设置为"all"
        text.group_id = group_id.unwrap_or_else(|| "all".to_string());
        text.updated_at = chrono::Utc::now().timestamp();
        let updated_text = text.clone();
        save_quick_texts(&texts);
        Ok(updated_text)
    } else {
        Err("常用文本不存在".to_string())
    }
}

// 删除常用文本
pub fn delete_quick_text(id: String) -> Result<(), String> {
    let mut texts = QUICK_TEXTS.lock().unwrap();

    if texts.remove(&id).is_some() {
        save_quick_texts(&texts);
        Ok(())
    } else {
        Err("常用文本不存在".to_string())
    }
}

// 根据ID获取常用文本
pub fn get_quick_text_by_id(id: String) -> Option<QuickText> {
    let texts = QUICK_TEXTS.lock().unwrap();
    texts.get(&id).cloned()
}

// 重新排序常用文本
pub fn reorder_quick_texts(items: Vec<QuickText>) -> Result<(), String> {
    let mut texts = QUICK_TEXTS.lock().unwrap();

    // 验证传入的项目，确保它们都存在且 group_id 一致
    let mut group_id_check: Option<String> = None;
    for item in &items {
        if let Some(existing_text) = texts.get(&item.id) {
            if group_id_check.is_none() {
                group_id_check = Some(existing_text.group_id.clone());
            } else if group_id_check.as_ref() != Some(&existing_text.group_id) {
                return Err("不能跨分组排序常用文本".to_string());
            }
        } else {
            return Err(format!("常用文本 {} 不存在", item.id));
        }
    }

    // 只更新排序时间戳，保持所有其他字段不变
    let now = chrono::Utc::now().timestamp();
    for (index, item) in items.iter().enumerate() {
        if let Some(existing_text) = texts.get_mut(&item.id) {
            // 设置时间戳：第一个项目时间戳最大，最后一个项目时间戳最小
            // 因为获取数据时是按 updated_at 倒序排列的
            existing_text.updated_at = now + (items.len() - index) as i64;
        }
    }

    save_quick_texts(&texts);

    let group_name = group_id_check.unwrap_or_else(|| "未知".to_string());
    println!(
        "已重新排序分组 {} 中的 {} 个常用文本",
        group_name,
        items.len()
    );
    Ok(())
}

// 移动常用文本到指定分组
pub fn move_quick_text_to_group(id: String, group_id: String) -> Result<(), String> {
    let mut texts = QUICK_TEXTS.lock().unwrap();

    if let Some(text) = texts.get_mut(&id) {
        let old_group_id = text.group_id.clone();
        text.group_id = group_id.clone();
        text.updated_at = chrono::Utc::now().timestamp();

        // 在锁内保存，确保原子性
        save_quick_texts(&texts);

        println!(
            "已将常用文本 {} 从分组 {} 移动到分组 {}",
            id, old_group_id, group_id
        );
        Ok(())
    } else {
        Err(format!("常用文本 {} 不存在", id))
    }
}

// 将指定分组中的所有常用文本移动到"全部"分组
pub fn move_group_texts_to_all(group_id: &str) -> Result<(), String> {
    let mut texts = QUICK_TEXTS.lock().unwrap();
    let mut changed = false;
    let mut moved_count = 0;

    for text in texts.values_mut() {
        if text.group_id == group_id {
            text.group_id = "all".to_string();
            text.updated_at = chrono::Utc::now().timestamp();
            changed = true;
            moved_count += 1;
        }
    }

    if changed {
        save_quick_texts(&texts);
        println!(
            "已将 {} 个常用文本从分组 {} 移动到全部分组",
            moved_count, group_id
        );
    } else {
        println!("分组 {} 中没有常用文本需要移动", group_id);
    }

    Ok(())
}
