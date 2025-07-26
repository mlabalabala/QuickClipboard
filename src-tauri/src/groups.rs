use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub created_at: String,
    pub updated_at: String,
}

// 全局分组存储
static GROUPS: Lazy<Mutex<HashMap<String, Group>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static GROUP_ORDER: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

// 获取数据文件路径
fn get_groups_file_path() -> Result<PathBuf, String> {
    // 使用本地数据目录 (AppData\Local\quickclipboard)，与其他组件保持一致
    let mut path = dirs::data_local_dir().unwrap_or_else(|| {
        println!("警告: 无法获取本地数据目录，使用当前目录");
        PathBuf::from(".")
    });
    path.push("quickclipboard");

    if let Err(e) = std::fs::create_dir_all(&path) {
        return Err(format!("创建数据目录失败: {}", e));
    }

    path.push("groups.json");
    Ok(path)
}

// 保存分组到文件
fn save_groups_to_file() -> Result<(), String> {
    let groups = GROUPS.lock().unwrap();
    let order = GROUP_ORDER.lock().unwrap();

    let data = serde_json::json!({
        "groups": *groups,
        "order": *order
    });

    let file_path = get_groups_file_path()?;
    fs::write(file_path, serde_json::to_string_pretty(&data).unwrap())
        .map_err(|e| format!("保存分组失败: {}", e))
}

// 从文件加载分组
fn load_groups_from_file() -> Result<(), String> {
    let file_path = get_groups_file_path()?;

    if !file_path.exists() {
        // 如果文件不存在，初始化默认分组
        init_default_groups()?;
        return Ok(());
    }

    let content = fs::read_to_string(file_path).map_err(|e| format!("读取分组文件失败: {}", e))?;

    let data: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("解析分组文件失败: {}", e))?;

    if let Some(groups_obj) = data.get("groups").and_then(|v| v.as_object()) {
        let mut groups = GROUPS.lock().unwrap();
        groups.clear();

        for (id, group_value) in groups_obj {
            if let Ok(group) = serde_json::from_value::<Group>(group_value.clone()) {
                groups.insert(id.clone(), group);
            }
        }
    }

    if let Some(order_array) = data.get("order").and_then(|v| v.as_array()) {
        let mut order = GROUP_ORDER.lock().unwrap();
        order.clear();

        for item in order_array {
            if let Some(id) = item.as_str() {
                order.push(id.to_string());
            }
        }
    }

    Ok(())
}

// 初始化默认分组
fn init_default_groups() -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let default_group = Group {
        id: "all".to_string(),
        name: "全部".to_string(),
        icon: "ti ti-folder".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    let mut groups = GROUPS.lock().unwrap();
    let mut order = GROUP_ORDER.lock().unwrap();

    groups.insert("all".to_string(), default_group);
    order.push("all".to_string());

    drop(groups);
    drop(order);

    save_groups_to_file()
}

// 初始化分组系统
pub fn init_groups() -> Result<(), String> {
    load_groups_from_file()
}

// 获取所有分组
pub fn get_all_groups() -> Vec<Group> {
    let groups = GROUPS.lock().unwrap();
    let order = GROUP_ORDER.lock().unwrap();

    let mut result = Vec::new();

    // 按顺序返回分组
    for id in order.iter() {
        if let Some(group) = groups.get(id) {
            result.push(group.clone());
        }
    }

    // 添加任何不在顺序中的分组
    for (id, group) in groups.iter() {
        if !order.contains(id) {
            result.push(group.clone());
        }
    }

    result
}

// 添加分组
pub fn add_group(name: String, icon: String) -> Result<Group, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();

    let group = Group {
        id: id.clone(),
        name,
        icon,
        created_at: now.clone(),
        updated_at: now,
    };

    let mut groups = GROUPS.lock().unwrap();
    let mut order = GROUP_ORDER.lock().unwrap();

    groups.insert(id.clone(), group.clone());
    order.push(id);

    drop(groups);
    drop(order);

    save_groups_to_file()?;
    Ok(group)
}

// 更新分组
pub fn update_group(id: String, name: String, icon: String) -> Result<Group, String> {
    let mut groups = GROUPS.lock().unwrap();

    if let Some(group) = groups.get_mut(&id) {
        group.name = name;
        group.icon = icon;
        group.updated_at = chrono::Utc::now().to_rfc3339();

        let updated_group = group.clone();
        drop(groups);

        save_groups_to_file()?;
        Ok(updated_group)
    } else {
        Err(format!("分组 {} 不存在", id))
    }
}

// 删除分组
pub fn delete_group(id: String) -> Result<(), String> {
    if id == "all" {
        return Err("不能删除默认分组".to_string());
    }

    let mut groups = GROUPS.lock().unwrap();
    let mut order = GROUP_ORDER.lock().unwrap();

    if groups.remove(&id).is_some() {
        order.retain(|x| x != &id);
        drop(groups);
        drop(order);

        // 将该分组中的常用文本移动到"全部"分组
        crate::quick_texts::move_group_texts_to_all(&id)?;

        save_groups_to_file()?;
        Ok(())
    } else {
        Err(format!("分组 {} 不存在", id))
    }
}

// 获取分组
pub fn get_group_by_id(id: &str) -> Option<Group> {
    let groups = GROUPS.lock().unwrap();
    groups.get(id).cloned()
}

// 检查分组是否存在
pub fn group_exists(id: &str) -> bool {
    let groups = GROUPS.lock().unwrap();
    groups.contains_key(id)
}
