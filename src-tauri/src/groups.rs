use crate::database;

// 使用database模块中的GroupInfo结构
pub use crate::database::GroupInfo;

// 确保"全部"分组正确存在并清理重复数据
fn ensure_all_group_exists() -> Result<String, String> {
    // 首先检查groups表中是否已有"全部"分组
    let conn_arc = crate::database::DB_CONNECTION.clone();
    let mut conn_guard = conn_arc
        .lock()
        .map_err(|e| format!("获取数据库锁失败: {}", e))?;
    
    match conn_guard.as_mut() {
        Some(conn) => {
            // 检查groups表中的"全部"分组
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM groups WHERE name = '全部'",
                [],
                |row| row.get(0),
            ).map_err(|e| format!("查询groups表失败: {}", e))?;
            
            if count == 0 {
                // groups表中没有"全部"分组，创建它
                let now = chrono::Utc::now().timestamp();
                conn.execute(
                    "INSERT INTO groups (name, icon, order_index, created_at, updated_at) VALUES ('全部', 'ti ti-list', -1, ?1, ?2)",
                    rusqlite::params![now, now],
                ).map_err(|e| format!("创建'全部'分组失败: {}", e))?;
                
                Ok("已创建'全部'分组".to_string())
            } else {
                // groups表中已有"全部"分组，确保它的order是-1（最高优先级）
                conn.execute(
                    "UPDATE groups SET order_index = -1, icon = 'ti ti-list' WHERE name = '全部'",
                    [],
                ).map_err(|e| format!("更新'全部'分组失败: {}", e))?;
                
                Ok("'全部'分组已存在并已更新".to_string())
            }
        }
        None => Err("数据库未初始化".to_string()),
    }
}

// 初始化分组系统
pub fn init_groups() -> Result<(), String> {
    println!("开始初始化分组系统...");
    
    // 确保"全部"分组存在于groups表中，并清理重复数据
    match ensure_all_group_exists() {
        Ok(msg) => println!("{}", msg),
        Err(e) => println!("处理'全部'分组时出错: {}", e),
    }
    
    println!("分组系统初始化完成");
    Ok(())
}

// 添加新分组
pub fn add_group(name: String, icon: String) -> Result<GroupInfo, String> {
    // 添加到groups表
    crate::database::create_group(&name, &icon)?;
    
    let group = GroupInfo {
        name,
        icon,
        order: 0,
        item_count: 0,
    };
    
    println!("分组已创建: {}", group.name);
    Ok(group)
}

// 更新分组
pub fn update_group(id: String, name: String, icon: String) -> Result<GroupInfo, String> {
    database::update_group_info(&id, &name, &icon, 0)?;
    
    let updated_group = GroupInfo {
        name,
        icon,
        order: 0,
        item_count: 0,
    };
    
    println!("分组已更新");
    Ok(updated_group)
}

// 删除分组
pub fn delete_group(id: String) -> Result<(), String> {
    database::delete_group_items(&id)?;
    
    println!("分组已删除，相关项目已移动到全部");
    Ok(())
}