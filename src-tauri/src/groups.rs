use crate::database;
use uuid::Uuid;

// 使用database模块中的Group结构
pub use crate::database::Group;

// 初始化分组系统
pub fn init_groups() -> Result<(), String> {
    println!("开始初始化分组系统...");
    
    // 从数据库加载分组
    let db_groups = database::get_all_groups()?;
    
    println!("从数据库获取到 {} 个分组", db_groups.len());
    
    // 如果没有分组，初始化默认分组
    if db_groups.is_empty() {
        println!("数据库中没有分组，初始化默认分组...");
        init_default_groups_db()?;
    } else {
        println!("从数据库加载了 {} 个分组", db_groups.len());
    }
    
    Ok(())
}

// 在数据库中初始化默认分组
fn init_default_groups_db() -> Result<(), String> {
    let now = chrono::Local::now().to_rfc3339();
    let default_group = Group {
        id: "all".to_string(),
        name: "全部".to_string(),
        icon: "ti ti-folder".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };

    // 保存到数据库
    database::add_group(&default_group)?;
    database::set_group_order(&["all".to_string()])?;
    
    println!("已初始化默认分组到数据库");
    Ok(())
}

// 获取所有分组
pub fn get_all_groups() -> Vec<Group> {
    match get_all_groups_from_db() {
        Ok(groups) => groups,
        Err(e) => {
            println!("从数据库获取分组失败: {}", e);
            Vec::new()
        }
    }
}

// 从数据库获取所有分组
fn get_all_groups_from_db() -> Result<Vec<Group>, String> {
    let db_groups = database::get_all_groups()?;
    let db_order = database::get_group_order()?;
    
    let mut groups_map = std::collections::HashMap::new();
    for group in db_groups {
        groups_map.insert(group.id.clone(), group);
    }
    
    let mut result = Vec::new();
    
    // 按顺序返回分组
    for id in db_order.iter() {
        if let Some(group) = groups_map.get(id) {
            result.push(group.clone());
        }
    }
    
    // 添加任何不在顺序中的分组
    for (id, group) in groups_map.iter() {
        if !db_order.contains(id) {
            result.push(group.clone());
        }
    }
    
    Ok(result)
}

// 根据ID获取分组
pub fn get_group_by_id(id: &str) -> Option<Group> {
    match database::get_all_groups() {
        Ok(groups) => groups.into_iter().find(|g| g.id == id),
        Err(_) => None,
    }
}

// 添加分组
pub fn add_group(name: String, icon: String) -> Result<Group, String> {
    let now = chrono::Local::now().to_rfc3339();
    let id = Uuid::new_v4().to_string();

    let group = Group {
        id: id.clone(),
        name,
        icon,
        created_at: now.clone(),
        updated_at: now,
    };

    // 保存到数据库
    database::add_group(&group)?;
    
    // 更新分组顺序
    let current_order = database::get_group_order().unwrap_or_default();
    let mut new_order = current_order;
    new_order.push(id.clone());
    
    if let Err(e) = database::set_group_order(&new_order) {
        println!("更新分组顺序失败: {}", e);
    }
    
    println!("分组已保存到数据库");
    Ok(group)
}

// 更新分组
pub fn update_group(id: String, name: String, icon: String) -> Result<Group, String> {
    let now = chrono::Local::now().to_rfc3339();
    let updated_group = Group {
        id: id.clone(),
        name,
        icon,
        created_at: "".to_string(),
        updated_at: now,
    };

    // 在数据库中更新
    database::update_group(&updated_group)?;
    println!("分组已在数据库中更新");
    
    Ok(updated_group)
}

// 删除分组
pub fn delete_group(id: String) -> Result<(), String> {
    if id == "all" {
        return Err("不能删除默认分组".to_string());
    }

    // 从数据库删除
    database::delete_group(&id)?;
    
    // 更新分组顺序
    let current_order = database::get_group_order().unwrap_or_default();
    let new_order: Vec<String> = current_order.into_iter().filter(|x| x != &id).collect();
    
    if let Err(e) = database::set_group_order(&new_order) {
        println!("更新分组顺序失败: {}", e);
    }
    
    // 将该分组中的常用文本移动到"全部"分组
    crate::quick_texts::move_group_texts_to_all(&id)?;
    
    println!("分组已从数据库中删除");
    Ok(())
}

// 检查分组是否存在
pub fn group_exists(id: &str) -> bool {
    database::group_exists(id).unwrap_or(false)
}

// 获取分组顺序
pub fn get_group_order() -> Vec<String> {
    database::get_group_order().unwrap_or_default()
}

// 设置分组顺序
pub fn set_group_order(order: Vec<String>) -> Result<(), String> {
    database::set_group_order(&order)?;
    println!("分组顺序已更新");
    Ok(())
}

// 重新排序分组
pub fn reorder_groups(group_ids: Vec<String>) -> Result<(), String> {
    // 验证所有分组都存在
    for id in &group_ids {
        if !group_exists(id) {
            return Err(format!("分组 {} 不存在", id));
        }
    }
    
    set_group_order(group_ids)
}

// 获取分组统计信息
pub fn get_group_stats() -> (usize, usize) {
    match database::get_all_groups() {
        Ok(groups) => {
            let total_groups = groups.len();
            let non_default_groups = groups.iter().filter(|g| g.id != "all").count();
            (total_groups, non_default_groups)
        }
        Err(_) => (0, 0),
    }
}

// 获取分组中的常用文本数量
pub fn get_group_text_count(group_id: &str) -> usize {
    crate::quick_texts::get_group_text_count(group_id)
}

// 搜索分组
pub fn search_groups(query: &str) -> Vec<Group> {
    match database::get_all_groups() {
        Ok(groups) => {
            groups.into_iter()
                .filter(|group| {
                    group.name.to_lowercase().contains(&query.to_lowercase())
                })
                .collect()
        }
        Err(_) => Vec::new(),
    }
}

// 验证分组名称是否唯一
pub fn is_group_name_unique(name: &str, exclude_id: Option<&str>) -> bool {
    match database::get_all_groups() {
        Ok(groups) => {
            !groups.iter().any(|g| {
                g.name == name && exclude_id.map_or(true, |id| g.id != id)
            })
        }
        Err(_) => true, // 如果无法获取分组，假设名称唯一
    }
}

// 获取默认分组
pub fn get_default_group() -> Option<Group> {
    get_group_by_id("all")
}

// 确保默认分组存在
pub fn ensure_default_group() -> Result<(), String> {
    if !group_exists("all") {
        init_default_groups_db()?;
    }
    Ok(())
}
