use once_cell::sync::Lazy;
use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// 数据库连接池
pub static DB_CONNECTION: Lazy<Arc<Mutex<Option<Connection>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

// 数据库文件路径
pub static DB_FILE: Lazy<PathBuf> = Lazy::new(|| {
    let mut dir = dirs::data_local_dir().unwrap_or_else(|| std::env::temp_dir());
    dir.push("quickclipboard");
    let _ = std::fs::create_dir_all(&dir);
    dir.push("clipboard.db");
    dir
});

// 剪贴板项目数据结构（与现有结构兼容）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: i64,
    pub text: String,
    pub is_image: bool,
    pub image_id: Option<String>,
    pub timestamp: u64,
    pub created_at: Option<String>, // DATETIME字段，可能为空（兼容旧数据）
}

impl ClipboardItem {
    pub fn new_text(text: String) -> Self {
        let now = chrono::Local::now();
        let local_timestamp = now.timestamp() as u64;
        
        Self {
            id: 0, // 将由数据库自动分配
            text,
            is_image: false,
            image_id: None,
            timestamp: local_timestamp,
            created_at: None, // 将由数据库填充
        }
    }

    pub fn new_image(image_id: String) -> Self {
        let now = chrono::Local::now();
        let local_timestamp = now.timestamp() as u64;
        
        Self {
            id: 0, // 将由数据库自动分配
            text: format!("image:{}", image_id),
            is_image: true,
            image_id: Some(image_id),
            timestamp: local_timestamp,
            created_at: None, // 将由数据库填充
        }
    }
}

// 常用文本数据结构
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuickText {
    pub id: String,
    pub title: String,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub group_id: String,
}

// 分组数据结构
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub created_at: String,
    pub updated_at: String,
}

// 初始化数据库
pub fn initialize_database() -> SqliteResult<()> {
    let db_path = &*DB_FILE;
    println!("初始化数据库: {:?}", db_path);

    let conn = Connection::open(db_path)?;

    // 创建表
    create_tables(&conn)?;

    // 存储连接
    let mut db_conn = DB_CONNECTION.lock().unwrap();
    *db_conn = Some(conn);

    println!("数据库初始化完成");
    Ok(())
}

// 创建数据库表
fn create_tables(conn: &Connection) -> SqliteResult<()> {
    // 剪贴板历史表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            is_image BOOLEAN NOT NULL DEFAULT 0,
            image_id TEXT,
            timestamp INTEGER NOT NULL,
            created_at DATETIME DEFAULT (DATETIME('now', 'localtime'))
        )",
        [],
    )?;

    // 常用文本表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS quick_texts (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            group_id TEXT NOT NULL DEFAULT 'all'
        )",
        [],
    )?;

    // 分组表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS groups (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            icon TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    )?;

    // 分组顺序表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS group_order (
            position INTEGER PRIMARY KEY,
            group_id TEXT NOT NULL
        )",
        [],
    )?;

    // 设置表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at DATETIME DEFAULT (DATETIME('now', 'localtime'))
        )",
        [],
    )?;

    // 创建索引
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_clipboard_timestamp ON clipboard_items(timestamp DESC)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_quick_texts_group ON quick_texts(group_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_quick_texts_updated ON quick_texts(updated_at DESC)",
        [],
    )?;

    Ok(())
}

// 获取数据库连接
pub fn get_connection() -> Result<Arc<Mutex<Option<Connection>>>, String> {
    Ok(DB_CONNECTION.clone())
}

// 关闭数据库连接
pub fn close_database_connection() -> Result<(), String> {
    let mut db_conn = DB_CONNECTION
        .lock()
        .map_err(|e| format!("获取数据库锁失败: {}", e))?;
    if db_conn.is_some() {
        *db_conn = None;
        println!("数据库连接已关闭");
    }
    Ok(())
}

// 执行数据库操作的辅助函数
pub fn with_connection<F, R>(f: F) -> Result<R, String>
where
    F: FnOnce(&Connection) -> SqliteResult<R>,
{
    let conn_arc = DB_CONNECTION.clone();
    let conn_guard = conn_arc
        .lock()
        .map_err(|e| format!("获取数据库锁失败: {}", e))?;

    match conn_guard.as_ref() {
        Some(conn) => f(conn).map_err(|e| format!("数据库操作失败: {}", e)),
        None => Err("数据库未初始化".to_string()),
    }
}

// 数据迁移功能
pub fn migrate_from_json() -> Result<(), String> {
    println!("开始检查JSON数据迁移...");

    // 检查是否需要迁移
    let needs_migration = with_connection(|conn| {
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM clipboard_items", [], |row| row.get(0))?;
        Ok(count == 0)
    })?;

    if !needs_migration {
        println!("数据库已有数据，跳过迁移");
        return Ok(());
    }

    // 迁移剪贴板历史
    migrate_clipboard_history()?;

    // 迁移常用文本
    migrate_quick_texts()?;

    // 迁移分组
    migrate_groups()?;

    println!("JSON数据迁移完成");
    Ok(())
}

// 迁移剪贴板历史
fn migrate_clipboard_history() -> Result<(), String> {
    let history_file = get_history_json_path();

    if !history_file.exists() {
        println!("未找到剪贴板历史JSON文件，跳过迁移");
        return Ok(());
    }

    println!("迁移剪贴板历史数据...");

    let content =
        std::fs::read_to_string(&history_file).map_err(|e| format!("读取历史文件失败: {}", e))?;

    let history_list: Vec<String> =
        serde_json::from_str(&content).map_err(|e| format!("解析历史文件失败: {}", e))?;

    with_connection(|conn| {
        let tx = conn.unchecked_transaction()?;

        for (index, text) in history_list.iter().enumerate() {
            let now = chrono::Local::now();
            let base_timestamp = now.timestamp() as u64;
            let timestamp = base_timestamp - (history_list.len() - index) as u64; // 保持相对顺序

            let (is_image, image_id) = if text.starts_with("image:") {
                (
                    true,
                    Some(text.strip_prefix("image:").unwrap_or("").to_string()),
                )
            } else {
                (false, None)
            };

            tx.execute(
                "INSERT INTO clipboard_items (text, is_image, image_id, timestamp) VALUES (?1, ?2, ?3, ?4)",
                params![text, is_image, image_id, timestamp],
            )?;
        }

        tx.commit()?;
        Ok(())
    })?;

    println!("剪贴板历史迁移完成，共迁移 {} 条记录", history_list.len());
    Ok(())
}

// 迁移常用文本
fn migrate_quick_texts() -> Result<(), String> {
    let quick_texts_file = get_quick_texts_json_path();

    if !quick_texts_file.exists() {
        println!("未找到常用文本JSON文件，跳过迁移");
        return Ok(());
    }

    println!("迁移常用文本数据...");

    let content = std::fs::read_to_string(&quick_texts_file)
        .map_err(|e| format!("读取常用文本文件失败: {}", e))?;

    let quick_texts: Vec<QuickText> =
        serde_json::from_str(&content).map_err(|e| format!("解析常用文本文件失败: {}", e))?;

    with_connection(|conn| {
        let tx = conn.unchecked_transaction()?;

        for text in &quick_texts {
            tx.execute(
                "INSERT INTO quick_texts (id, title, content, created_at, updated_at, group_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![text.id, text.title, text.content, text.created_at, text.updated_at, text.group_id],
            )?;
        }

        tx.commit()?;
        Ok(())
    })?;

    println!("常用文本迁移完成，共迁移 {} 条记录", quick_texts.len());
    Ok(())
}

// 迁移分组数据
fn migrate_groups() -> Result<(), String> {
    let groups_file = get_groups_json_path();

    if !groups_file.exists() {
        println!("未找到分组JSON文件，跳过迁移");
        return Ok(());
    }

    println!("迁移分组数据...");

    let content =
        std::fs::read_to_string(&groups_file).map_err(|e| format!("读取分组文件失败: {}", e))?;

    let data: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("解析分组文件失败: {}", e))?;

    with_connection(|conn| {
        let tx = conn.unchecked_transaction()?;

        // 迁移分组
        if let Some(groups_obj) = data.get("groups").and_then(|v| v.as_object()) {
            for (id, group_data) in groups_obj {
                if let Ok(group) = serde_json::from_value::<Group>(group_data.clone()) {
                    tx.execute(
                        "INSERT INTO groups (id, name, icon, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![group.id, group.name, group.icon, group.created_at, group.updated_at],
                    )?;
                }
            }
        }

        // 迁移分组顺序
        if let Some(order_array) = data.get("order").and_then(|v| v.as_array()) {
            for (position, item) in order_array.iter().enumerate() {
                if let Some(group_id) = item.as_str() {
                    tx.execute(
                        "INSERT INTO group_order (position, group_id) VALUES (?1, ?2)",
                        params![position as i32, group_id],
                    )?;
                }
            }
        }

        tx.commit()?;
        Ok(())
    })?;

    println!("分组数据迁移完成");
    Ok(())
}

// 获取JSON文件路径的辅助函数
fn get_history_json_path() -> PathBuf {
    let mut dir = dirs::data_local_dir().unwrap_or_else(|| std::env::temp_dir());
    dir.push("quickclipboard");
    dir.push("history.json");
    dir
}

fn get_quick_texts_json_path() -> PathBuf {
    let mut dir = dirs::data_local_dir().unwrap_or_else(|| std::env::temp_dir());
    dir.push("quickclipboard");
    dir.push("quick_texts.json");
    dir
}

fn get_groups_json_path() -> PathBuf {
    let mut dir = dirs::data_local_dir().unwrap_or_else(|| std::env::temp_dir());
    dir.push("quickclipboard");
    dir.push("groups.json");
    dir
}

// =================== 剪贴板历史数据库操作 ===================

// 添加剪贴板项目
pub fn add_clipboard_item(text: String) -> Result<i64, String> {
    let item = ClipboardItem::new_text(text);
    let now_local = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    with_connection(|conn| {
        conn.execute(
            "INSERT INTO clipboard_items (text, is_image, image_id, timestamp, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![item.text, item.is_image, item.image_id, item.timestamp, now_local],
        )?;

        Ok(conn.last_insert_rowid())
    })
}

// 添加图片剪贴板项目
pub fn add_clipboard_image_item(image_id: String) -> Result<i64, String> {
    let item = ClipboardItem::new_image(image_id);
    let now_local = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    with_connection(|conn| {
        conn.execute(
            "INSERT INTO clipboard_items (text, is_image, image_id, timestamp, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![item.text, item.is_image, item.image_id, item.timestamp, now_local],
        )?;

        Ok(conn.last_insert_rowid())
    })
}

// 获取剪贴板历史（按时间戳倒序）
pub fn get_clipboard_history(limit: Option<usize>) -> Result<Vec<ClipboardItem>, String> {
    with_connection(|conn| {
        let sql = if let Some(limit) = limit {
            format!("SELECT id, text, is_image, image_id, timestamp, created_at FROM clipboard_items ORDER BY timestamp DESC LIMIT {}", limit)
        } else {
            "SELECT id, text, is_image, image_id, timestamp, created_at FROM clipboard_items ORDER BY timestamp DESC".to_string()
        };

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            Ok(ClipboardItem {
                id: row.get(0)?,
                text: row.get(1)?,
                is_image: row.get(2)?,
                image_id: row.get(3)?,
                timestamp: row.get(4)?,
                created_at: row.get(5).ok(), // 使用 .ok() 处理可能的 NULL 值
            })
        })?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }

        Ok(items)
    })
}

// 检查剪贴板项目是否存在
pub fn clipboard_item_exists(text: &str) -> Result<Option<i64>, String> {
    with_connection(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id FROM clipboard_items WHERE text = ?1 ORDER BY timestamp DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map([text], |row| Ok(row.get::<_, i64>(0)?))?;

        if let Some(row) = rows.next() {
            Ok(Some(row?))
        } else {
            Ok(None)
        }
    })
}

// 移动剪贴板项目到最前面（更新时间戳）
pub fn move_clipboard_item_to_front(id: i64) -> Result<(), String> {
    let now = chrono::Local::now();
    let new_timestamp = now.timestamp() as u64;

    with_connection(|conn| {
        conn.execute(
            "UPDATE clipboard_items SET timestamp = ?1 WHERE id = ?2",
            params![new_timestamp, id],
        )?;
        Ok(())
    })
}

// 删除剪贴板项目
pub fn delete_clipboard_item(id: i64) -> Result<(), String> {
    with_connection(|conn| {
        conn.execute("DELETE FROM clipboard_items WHERE id = ?1", params![id])?;
        Ok(())
    })
}

// 更新剪贴板项目内容
pub fn update_clipboard_item(id: i64, new_text: String) -> Result<(), String> {
    with_connection(|conn| {
        conn.execute(
            "UPDATE clipboard_items SET text = ?1 WHERE id = ?2",
            params![new_text, id],
        )?;
        Ok(())
    })
}

// 清空剪贴板历史
pub fn clear_clipboard_history() -> Result<(), String> {
    with_connection(|conn| {
        conn.execute("DELETE FROM clipboard_items", [])?;
        Ok(())
    })
}

// 限制剪贴板历史数量
pub fn limit_clipboard_history(max_count: usize) -> Result<(), String> {
    with_connection(|conn| {
        // 删除超出限制的旧记录
        conn.execute(
            "DELETE FROM clipboard_items WHERE id NOT IN (
                SELECT id FROM clipboard_items ORDER BY timestamp DESC LIMIT ?1
            )",
            params![max_count],
        )?;
        Ok(())
    })
}

// 批量更新剪贴板项目的时间戳（用于重新排序）
pub fn reorder_clipboard_items(texts: &[String]) -> Result<(), String> {
    with_connection(|conn| {
        let tx = conn.unchecked_transaction()?;

        // 获取当前最大时间戳，确保排序后的项目时间戳都小于未来可能的新项目
        let max_timestamp: u64 = conn.query_row(
            "SELECT COALESCE(MAX(timestamp), 0) FROM clipboard_items",
            [],
            |row| row.get(0),
        )?;

        // 使用比当前最大时间戳小的值作为基准，为排序项目分配时间戳
        // 这样确保新复制的内容（使用当前时间戳）总是在最前面
        let base_timestamp = max_timestamp.saturating_sub(texts.len() as u64 * 2);

        // 为每个文本分配新的时间戳，第一个项目时间戳最大（但仍小于未来的新项目）
        for (index, text) in texts.iter().enumerate() {
            let new_timestamp = base_timestamp + (texts.len() - index) as u64;

            tx.execute(
                "UPDATE clipboard_items SET timestamp = ?1 WHERE text = ?2",
                params![new_timestamp, text],
            )?;
        }

        tx.commit()?;
        Ok(())
    })
}

// =================== 常用文本数据库操作 ===================

// 添加常用文本
pub fn add_quick_text(quick_text: &QuickText) -> Result<(), String> {
    with_connection(|conn| {
        conn.execute(
            "INSERT INTO quick_texts (id, title, content, created_at, updated_at, group_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![quick_text.id, quick_text.title, quick_text.content, quick_text.created_at, quick_text.updated_at, quick_text.group_id],
        )?;
        Ok(())
    })
}

// 获取所有常用文本
pub fn get_all_quick_texts() -> Result<Vec<QuickText>, String> {
    with_connection(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, title, content, created_at, updated_at, group_id FROM quick_texts ORDER BY updated_at DESC"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(QuickText {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                group_id: row.get(5)?,
            })
        })?;

        let mut texts = Vec::new();
        for row in rows {
            texts.push(row?);
        }

        Ok(texts)
    })
}

// 按分组获取常用文本
pub fn get_quick_texts_by_group(group_id: &str) -> Result<Vec<QuickText>, String> {
    with_connection(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, title, content, created_at, updated_at, group_id FROM quick_texts WHERE group_id = ?1 ORDER BY updated_at DESC"
        )?;

        let rows = stmt.query_map([group_id], |row| {
            Ok(QuickText {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                group_id: row.get(5)?,
            })
        })?;

        let mut texts = Vec::new();
        for row in rows {
            texts.push(row?);
        }

        Ok(texts)
    })
}

// 更新常用文本
pub fn update_quick_text(quick_text: &QuickText) -> Result<(), String> {
    with_connection(|conn| {
        conn.execute(
            "UPDATE quick_texts SET title = ?1, content = ?2, updated_at = ?3, group_id = ?4 WHERE id = ?5",
            params![quick_text.title, quick_text.content, quick_text.updated_at, quick_text.group_id, quick_text.id],
        )?;
        Ok(())
    })
}

// 删除常用文本
pub fn delete_quick_text(id: &str) -> Result<(), String> {
    with_connection(|conn| {
        conn.execute("DELETE FROM quick_texts WHERE id = ?1", params![id])?;
        Ok(())
    })
}

// 检查常用文本是否存在
pub fn quick_text_exists(id: &str) -> Result<bool, String> {
    with_connection(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM quick_texts WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    })
}

// 批量更新常用文本的时间戳（用于重新排序）
pub fn reorder_quick_texts(items: &[QuickText]) -> Result<(), String> {
    with_connection(|conn| {
        let tx = conn.unchecked_transaction()?;

        // 获取当前最大时间戳，确保排序后的项目时间戳都小于未来可能的新项目
        let max_timestamp: i64 = conn.query_row(
            "SELECT COALESCE(MAX(updated_at), 0) FROM quick_texts",
            [],
            |row| row.get(0),
        )?;

        // 使用比当前最大时间戳小的值作为基准，为排序项目分配时间戳
        // 这样确保新添加的内容（使用当前时间戳）总是在最前面
        let base_timestamp = max_timestamp.saturating_sub(items.len() as i64 * 2);

        // 为每个项目分配新的时间戳，第一个项目时间戳最大（但仍小于未来的新项目）
        for (index, item) in items.iter().enumerate() {
            let new_timestamp = base_timestamp + (items.len() - index) as i64;

            tx.execute(
                "UPDATE quick_texts SET updated_at = ?1 WHERE id = ?2",
                params![new_timestamp, item.id],
            )?;
        }

        tx.commit()?;
        Ok(())
    })
}

// =================== 分组数据库操作 ===================

// 添加分组
pub fn add_group(group: &Group) -> Result<(), String> {
    with_connection(|conn| {
        conn.execute(
            "INSERT INTO groups (id, name, icon, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![group.id, group.name, group.icon, group.created_at, group.updated_at],
        )?;
        Ok(())
    })
}

// 获取所有分组
pub fn get_all_groups() -> Result<Vec<Group>, String> {
    with_connection(|conn| {
        let mut stmt = conn.prepare("SELECT id, name, icon, created_at, updated_at FROM groups")?;

        let rows = stmt.query_map([], |row| {
            Ok(Group {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        let mut groups = Vec::new();
        for row in rows {
            groups.push(row?);
        }

        Ok(groups)
    })
}

// 更新分组
pub fn update_group(group: &Group) -> Result<(), String> {
    with_connection(|conn| {
        conn.execute(
            "UPDATE groups SET name = ?1, icon = ?2, updated_at = ?3 WHERE id = ?4",
            params![group.name, group.icon, group.updated_at, group.id],
        )?;
        Ok(())
    })
}

// 删除分组
pub fn delete_group(id: &str) -> Result<(), String> {
    with_connection(|conn| {
        conn.execute("DELETE FROM groups WHERE id = ?1", params![id])?;
        Ok(())
    })
}

// 检查分组是否存在
pub fn group_exists(id: &str) -> Result<bool, String> {
    with_connection(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM groups WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    })
}

// 获取分组顺序
pub fn get_group_order() -> Result<Vec<String>, String> {
    with_connection(|conn| {
        let mut stmt = conn.prepare("SELECT group_id FROM group_order ORDER BY position")?;

        let rows = stmt.query_map([], |row| Ok(row.get::<_, String>(0)?))?;

        let mut order = Vec::new();
        for row in rows {
            order.push(row?);
        }

        Ok(order)
    })
}

// 设置分组顺序
pub fn set_group_order(order: &[String]) -> Result<(), String> {
    with_connection(|conn| {
        let tx = conn.unchecked_transaction()?;

        // 清空现有顺序
        tx.execute("DELETE FROM group_order", [])?;

        // 插入新顺序
        for (position, group_id) in order.iter().enumerate() {
            tx.execute(
                "INSERT INTO group_order (position, group_id) VALUES (?1, ?2)",
                params![position as i32, group_id],
            )?;
        }

        tx.commit()?;
        Ok(())
    })
}

// 清空所有数据
pub fn clear_all_data() -> Result<(), String> {
    let conn_arc = DB_CONNECTION.clone();
    let mut conn_guard = conn_arc
        .lock()
        .map_err(|e| format!("获取数据库锁失败: {}", e))?;

    match conn_guard.as_mut() {
        Some(conn) => {
            let tx = conn
                .transaction()
                .map_err(|e| format!("创建事务失败: {}", e))?;

            // 安全地清空所有表（如果表存在的话）
            let tables = vec!["clipboard_history", "quick_texts", "groups", "group_order"];

            for table in tables {
                // 检查表是否存在
                let table_exists: bool = tx
                    .prepare(&format!(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'",
                        table
                    ))
                    .and_then(|mut stmt| {
                        stmt.query_row([], |row| {
                            let count: i64 = row.get(0)?;
                            Ok(count > 0)
                        })
                    })
                    .unwrap_or(false);

                if table_exists {
                    tx.execute(&format!("DELETE FROM {}", table), [])
                        .map_err(|e| format!("清空表 {} 失败: {}", table, e))?;
                }
            }

            // 重置自增ID（如果sqlite_sequence表存在）
            let sequence_exists: bool = tx
                .prepare("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sqlite_sequence'")
                .and_then(|mut stmt| {
                    stmt.query_row([], |row| {
                        let count: i64 = row.get(0)?;
                        Ok(count > 0)
                    })
                })
                .unwrap_or(false);

            if sequence_exists {
                tx.execute("DELETE FROM sqlite_sequence WHERE name IN ('clipboard_history', 'quick_texts', 'groups')", [])
                    .map_err(|e| format!("重置自增ID失败: {}", e))?;
            }

            tx.commit().map_err(|e| format!("提交事务失败: {}", e))?;
            Ok(())
        }
        None => Err("数据库未初始化".to_string()),
    }
}
