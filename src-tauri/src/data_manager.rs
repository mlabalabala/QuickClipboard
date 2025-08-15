use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::{write::FileOptions, ZipArchive, ZipWriter};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportOptions {
    pub clipboard_history: bool,
    pub quick_texts: bool,
    pub groups: bool,
    pub settings: bool,
    pub images: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportOptions {
    pub mode: ImportMode,
    pub clipboard_history: Option<bool>,
    pub quick_texts: Option<bool>,
    pub groups: Option<bool>,
    pub settings: Option<bool>,
    pub images: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ImportMode {
    Replace,
    Merge,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportMetadata {
    pub version: String,
    pub export_time: String,
    pub app_version: String,
    pub options: ExportOptions,
}

// 获取应用数据目录
pub fn get_app_data_dir() -> Result<PathBuf, String> {
    dirs::data_local_dir()
        .map(|dir| dir.join("quickclipboard"))
        .ok_or_else(|| "无法获取应用数据目录".to_string())
}

// 导出数据到ZIP文件
pub async fn export_data(export_path: &str, options: ExportOptions) -> Result<(), String> {
    let app_data_dir = get_app_data_dir()?;

    // 创建ZIP文件
    let file = fs::File::create(export_path).map_err(|e| format!("创建导出文件失败: {}", e))?;

    let mut zip = ZipWriter::new(file);
    let zip_options = FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    // 创建导出元数据
    let metadata = ExportMetadata {
        version: "1.0".to_string(),
        export_time: Local::now().to_rfc3339(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        options: options.clone(),
    };

    // 添加元数据文件
    zip.start_file("metadata.json", zip_options)
        .map_err(|e| format!("创建元数据文件失败: {}", e))?;

    let metadata_json =
        serde_json::to_string_pretty(&metadata).map_err(|e| format!("序列化元数据失败: {}", e))?;

    zip.write_all(metadata_json.as_bytes())
        .map_err(|e| format!("写入元数据失败: {}", e))?;

    // 导出数据库文件
    if options.clipboard_history || options.quick_texts || options.groups {
        let db_path = app_data_dir.join("clipboard.db");
        if db_path.exists() {
            add_file_to_zip(&mut zip, &db_path, "clipboard.db", zip_options)?;
        }
    }

    // 导出设置文件
    if options.settings {
        let settings_path = app_data_dir.join("settings.json");
        if settings_path.exists() {
            add_file_to_zip(&mut zip, &settings_path, "settings.json", zip_options)?;
        }
    }

    // 导出图片数据
    if options.images {
        let images_dir = app_data_dir.join("clipboard_images");
        if images_dir.exists() {
            add_directory_to_zip(&mut zip, &images_dir, "clipboard_images", zip_options)?;
        }
    }

    zip.finish()
        .map_err(|e| format!("完成ZIP文件创建失败: {}", e))?;

    Ok(())
}

// 导入数据从ZIP文件
pub async fn import_data(import_path: &str, options: ImportOptions) -> Result<(), String> {
    let app_data_dir = get_app_data_dir()?;

    // 打开ZIP文件
    let file = fs::File::open(import_path).map_err(|e| format!("打开导入文件失败: {}", e))?;

    let mut archive = ZipArchive::new(file).map_err(|e| format!("读取ZIP文件失败: {}", e))?;

    // 读取元数据
    let metadata = read_metadata_from_zip(&mut archive)?;

    let actual_options = ExportOptions {
        clipboard_history: options
            .clipboard_history
            .unwrap_or(metadata.options.clipboard_history),
        quick_texts: options.quick_texts.unwrap_or(metadata.options.quick_texts),
        groups: options.groups.unwrap_or(metadata.options.groups),
        settings: options.settings.unwrap_or(metadata.options.settings),
        images: options.images.unwrap_or(metadata.options.images),
    };

    // 根据导入模式处理数据
    match options.mode {
        ImportMode::Replace => {
            // 替换模式：先备份现有数据，然后清空
            backup_current_data(&app_data_dir).await?;
            clear_current_data(&app_data_dir, &actual_options).await?;
            // 提取文件（直接覆盖）
            extract_files_from_zip(&mut archive, &app_data_dir, &actual_options)?;
        }
        ImportMode::Merge => {
            // 合并模式：保留现有数据，进行数据合并
            backup_current_data(&app_data_dir).await?;
            // 合并数据而不是直接覆盖
            merge_data_from_zip(&mut archive, &app_data_dir, &actual_options).await?;
        }
    }

    Ok(())
}

// 清空剪贴板历史
pub async fn clear_clipboard_history() -> Result<(), String> {
    crate::clipboard_history::clear_all().map_err(|e| format!("清空剪贴板历史失败: {}", e))
}

// 重置所有数据
pub async fn reset_all_data() -> Result<(), String> {
    let app_data_dir = get_app_data_dir()?;

    // 备份当前数据
    backup_current_data(&app_data_dir).await?;

    // 关闭数据库连接以释放文件锁
    crate::database::close_database_connection()
        .map_err(|e| format!("关闭数据库连接失败: {}", e))?;

    // 删除所有数据文件
    let files_to_remove = vec!["clipboard.db", "settings.json"];

    for file in files_to_remove {
        let file_path = app_data_dir.join(file);
        if file_path.exists() {
            fs::remove_file(&file_path).map_err(|e| format!("删除文件 {} 失败: {}", file, e))?;
        }
    }

    // 删除图片目录
    let images_dir = app_data_dir.join("clipboard_images");
    if images_dir.exists() {
        fs::remove_dir_all(&images_dir).map_err(|e| format!("删除图片目录失败: {}", e))?;
    }

    Ok(())
}

// 辅助函数：添加文件到ZIP
fn add_file_to_zip(
    zip: &mut ZipWriter<fs::File>,
    file_path: &Path,
    zip_path: &str,
    options: FileOptions<()>,
) -> Result<(), String> {
    let mut file = fs::File::open(file_path)
        .map_err(|e| format!("打开文件 {} 失败: {}", file_path.display(), e))?;

    zip.start_file(zip_path, options)
        .map_err(|e| format!("在ZIP中创建文件 {} 失败: {}", zip_path, e))?;

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("读取文件 {} 失败: {}", file_path.display(), e))?;

    zip.write_all(&buffer)
        .map_err(|e| format!("写入ZIP文件 {} 失败: {}", zip_path, e))?;

    Ok(())
}

// 辅助函数：添加目录到ZIP（递归处理子目录）
fn add_directory_to_zip(
    zip: &mut ZipWriter<fs::File>,
    dir_path: &Path,
    zip_prefix: &str,
    options: FileOptions<()>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir_path)
        .map_err(|e| format!("读取目录 {} 失败: {}", dir_path.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let path = entry.path();

        if path.is_file() {
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| "无效的文件名".to_string())?;

            let zip_path = format!("{}/{}", zip_prefix, file_name);
            add_file_to_zip(zip, &path, &zip_path, options)?;
        } else if path.is_dir() {
            // 递归处理子目录
            let dir_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| "无效的目录名".to_string())?;

            let sub_zip_prefix = format!("{}/{}", zip_prefix, dir_name);
            add_directory_to_zip(zip, &path, &sub_zip_prefix, options)?;
        }
    }

    Ok(())
}

// 辅助函数：从ZIP读取元数据
fn read_metadata_from_zip(archive: &mut ZipArchive<fs::File>) -> Result<ExportMetadata, String> {
    let mut metadata_file = archive
        .by_name("metadata.json")
        .map_err(|e| format!("找不到元数据文件: {}", e))?;

    let mut metadata_content = String::new();
    metadata_file
        .read_to_string(&mut metadata_content)
        .map_err(|e| format!("读取元数据失败: {}", e))?;

    serde_json::from_str(&metadata_content).map_err(|e| format!("解析元数据失败: {}", e))
}

// 辅助函数：备份当前数据
async fn backup_current_data(app_data_dir: &Path) -> Result<(), String> {
    let backup_dir = app_data_dir.join("backup");
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let backup_path = backup_dir.join(format!("backup_{}.zip", timestamp));

    // 创建备份目录
    fs::create_dir_all(&backup_dir).map_err(|e| format!("创建备份目录失败: {}", e))?;

    // 导出当前数据作为备份
    let backup_options = ExportOptions {
        clipboard_history: true,
        quick_texts: true,
        groups: true,
        settings: true,
        images: true,
    };

    export_data(backup_path.to_str().unwrap(), backup_options).await?;

    Ok(())
}

// 辅助函数：清空当前数据
async fn clear_current_data(app_data_dir: &Path, options: &ExportOptions) -> Result<(), String> {
    if options.clipboard_history || options.quick_texts || options.groups {
        // 确保数据库已初始化
        crate::database::initialize_database().map_err(|e| format!("初始化数据库失败: {}", e))?;
        // 清空数据库
        crate::database::clear_all_data().map_err(|e| format!("清空数据库失败: {}", e))?;
    }

    if options.settings {
        let settings_path = app_data_dir.join("settings.json");
        if settings_path.exists() {
            fs::remove_file(&settings_path).map_err(|e| format!("删除设置文件失败: {}", e))?;
        }
    }

    if options.images {
        let images_dir = app_data_dir.join("clipboard_images");
        if images_dir.exists() {
            fs::remove_dir_all(&images_dir).map_err(|e| format!("删除图片目录失败: {}", e))?;
        }
    }

    Ok(())
}

// 辅助函数：从ZIP提取文件
fn extract_files_from_zip(
    archive: &mut ZipArchive<fs::File>,
    app_data_dir: &Path,
    options: &ExportOptions,
) -> Result<(), String> {
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("读取ZIP文件项失败: {}", e))?;

        let file_name = file.name();

        // 跳过元数据文件
        if file_name == "metadata.json" {
            continue;
        }

        // 根据选项决定是否提取文件
        let should_extract = match file_name {
            "clipboard.db" => options.clipboard_history || options.quick_texts || options.groups,
            "settings.json" => options.settings,
            name if name.starts_with("clipboard_images/") => options.images,
            _ => false,
        };

        if !should_extract {
            continue;
        }

        let output_path = app_data_dir.join(file_name);

        // 创建父目录
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
        }

        // 提取文件
        let mut output_file =
            fs::File::create(&output_path).map_err(|e| format!("创建输出文件失败: {}", e))?;

        std::io::copy(&mut file, &mut output_file).map_err(|e| format!("提取文件失败: {}", e))?;
    }

    Ok(())
}

// 合并数据从ZIP文件（用于合并模式）
async fn merge_data_from_zip(
    archive: &mut ZipArchive<fs::File>,
    app_data_dir: &Path,
    options: &ExportOptions,
) -> Result<(), String> {
    // 创建临时目录来提取导入的数据
    let temp_dir = app_data_dir.join("temp_import");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).map_err(|e| format!("清理临时目录失败: {}", e))?;
    }
    fs::create_dir_all(&temp_dir).map_err(|e| format!("创建临时目录失败: {}", e))?;

    // 先提取文件到临时目录
    extract_files_from_zip(archive, &temp_dir, options)?;

    // 处理数据库合并
    if options.clipboard_history || options.quick_texts || options.groups {
        let temp_db_path = temp_dir.join("clipboard.db");
        if temp_db_path.exists() {
            println!("开始合并数据库数据...");
            merge_database_data(&temp_db_path).await?;
            println!("数据库合并完成");
        } else {
            println!("临时数据库文件不存在: {:?}", temp_db_path);
        }
    }

    // 处理设置文件合并
    if options.settings {
        let temp_settings_path = temp_dir.join("settings.json");
        if temp_settings_path.exists() {
            merge_settings_data(&temp_settings_path, app_data_dir).await?;
        }
    }

    // 处理图片文件合并
    if options.images {
        let temp_images_dir = temp_dir.join("clipboard_images");
        if temp_images_dir.exists() {
            merge_images_data(&temp_images_dir, app_data_dir).await?;
        }
    }

    // 清理临时目录
    fs::remove_dir_all(&temp_dir).map_err(|e| format!("清理临时目录失败: {}", e))?;

    Ok(())
}

// 合并数据库数据
async fn merge_database_data(temp_db_path: &Path) -> Result<(), String> {
    use rusqlite::Connection;

    // 打开临时数据库
    let temp_conn =
        Connection::open(temp_db_path).map_err(|e| format!("打开临时数据库失败: {}", e))?;

    // 确保主数据库已初始化
    crate::database::initialize_database().map_err(|e| format!("初始化主数据库失败: {}", e))?;

    // 获取主数据库连接并执行合并操作
    let result = crate::database::with_connection(|main_conn| {
        // 合并剪贴板历史记录
        merge_clipboard_history(&temp_conn, main_conn)?;

        // 合并快速文本
        merge_quick_texts(&temp_conn, main_conn)?;

        // 合并分组
        merge_groups(&temp_conn, main_conn)?;

        Ok(())
    });

    result.map_err(|e| format!("数据库合并失败: {}", e))
}

// 合并剪贴板历史记录
fn merge_clipboard_history(
    temp_conn: &rusqlite::Connection,
    main_conn: &rusqlite::Connection,
) -> Result<(), rusqlite::Error> {
    use rusqlite::params;

    // 检查临时数据库中是否存在clipboard_items表
    let table_exists: bool = temp_conn.query_row(
        "SELECT EXISTS(SELECT name FROM sqlite_master WHERE type='table' AND name='clipboard_items')",
        [],
        |row| row.get(0)
    ).unwrap_or(false);

    if !table_exists {
        println!("clipboard_items表不存在，跳过合并");
        return Ok(());
    }

    // 查询clipboard_items表
    let mut stmt = temp_conn.prepare(
        "SELECT id, text, is_image, image_id, timestamp, created_at FROM clipboard_items",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,            // id
            row.get::<_, String>(1)?,         // text
            row.get::<_, bool>(2)?,           // is_image
            row.get::<_, Option<String>>(3)?, // image_id
            row.get::<_, i64>(4)?,            // timestamp
            row.get::<_, Option<String>>(5)?, // created_at
        ))
    })?;

    for row in rows {
        let (_id, text, is_image, image_id, timestamp, _created_at) = row?;

        // 检查是否存在相同内容的记录
        let content_exists: bool = main_conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM clipboard_items WHERE text = ?1 AND is_image = ?2)",
                params![text, is_image],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !content_exists {
            // 插入到clipboard_items表，让数据库自动分配新的ID
            main_conn.execute(
                "INSERT INTO clipboard_items (text, is_image, image_id, timestamp, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![text, is_image, image_id, timestamp, chrono::Local::now().to_rfc3339()],
            )?;
        }
    }

    Ok(())
}

// 合并快速文本
fn merge_quick_texts(
    temp_conn: &rusqlite::Connection,
    main_conn: &rusqlite::Connection,
) -> Result<(), rusqlite::Error> {
    use rusqlite::params;

    // 检查临时数据库中是否存在quick_texts表
    let table_exists: bool = temp_conn.query_row(
        "SELECT EXISTS(SELECT name FROM sqlite_master WHERE type='table' AND name='quick_texts')",
        [],
        |row| row.get(0)
    ).unwrap_or(false);

    if !table_exists {
        // 如果表不存在，直接返回成功（没有数据需要合并）
        return Ok(());
    }

    let mut stmt = temp_conn
        .prepare("SELECT id, title, content, group_id, created_at, updated_at FROM quick_texts")?;

    let rows = stmt.query_map([], |row| {
        // 灵活处理时间戳字段，支持字符串和整数类型
        let created_at = match row.get::<_, rusqlite::types::Value>(4)? {
            rusqlite::types::Value::Text(s) => s,
            rusqlite::types::Value::Integer(i) => i.to_string(),
            _ => chrono::Local::now().to_rfc3339(),
        };

        let updated_at = match row.get::<_, rusqlite::types::Value>(5)? {
            rusqlite::types::Value::Text(s) => s,
            rusqlite::types::Value::Integer(i) => i.to_string(),
            _ => chrono::Local::now().to_rfc3339(),
        };

        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            created_at,
            updated_at,
        ))
    })?;

    for row in rows {
        let (_id, title, content, group_id, created_at, updated_at) = row?;

        // 检查是否存在相同内容的记录
        let content_exists: bool = main_conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM quick_texts WHERE title = ?1 AND content = ?2)",
                params![title, content],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !content_exists {
            // 生成新的UUID作为ID，避免ID冲突
            let new_id = uuid::Uuid::new_v4().to_string();
            main_conn.execute(
                "INSERT INTO quick_texts (id, title, content, group_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![new_id, title, content, group_id, created_at, updated_at],
            )?;
        }
    }

    Ok(())
}

// 合并分组
fn merge_groups(
    temp_conn: &rusqlite::Connection,
    main_conn: &rusqlite::Connection,
) -> Result<(), rusqlite::Error> {
    use rusqlite::params;

    // 检查临时数据库中是否存在groups表
    let table_exists: bool = temp_conn
        .query_row(
            "SELECT EXISTS(SELECT name FROM sqlite_master WHERE type='table' AND name='groups')",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !table_exists {
        // 如果表不存在，直接返回成功（没有数据需要合并）
        return Ok(());
    }

    let mut stmt =
        temp_conn.prepare("SELECT id, name, icon, created_at, updated_at FROM groups")?;

    let rows = stmt.query_map([], |row| {
        // 灵活处理时间戳字段，支持字符串和整数类型
        let created_at = match row.get::<_, rusqlite::types::Value>(3)? {
            rusqlite::types::Value::Text(s) => s,
            rusqlite::types::Value::Integer(i) => i.to_string(),
            _ => chrono::Local::now().to_rfc3339(),
        };

        let updated_at = match row.get::<_, rusqlite::types::Value>(4)? {
            rusqlite::types::Value::Text(s) => s,
            rusqlite::types::Value::Integer(i) => i.to_string(),
            _ => chrono::Local::now().to_rfc3339(),
        };

        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            created_at,
            updated_at,
        ))
    })?;

    for row in rows {
        let (_id, name, icon, created_at, updated_at) = row?;

        // 检查是否存在相同名称的分组
        let name_exists: bool = main_conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM groups WHERE name = ?1)",
                params![name],
                |row| row.get(0),
            )
            .unwrap_or(false);

        if !name_exists {
            // 生成新的UUID作为ID，避免ID冲突
            let new_id = uuid::Uuid::new_v4().to_string();
            main_conn.execute(
                "INSERT INTO groups (id, name, icon, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![new_id, name, icon, created_at, updated_at],
            )?;
        }
    }

    Ok(())
}

// 合并设置数据
async fn merge_settings_data(temp_settings_path: &Path, app_data_dir: &Path) -> Result<(), String> {
    // 读取导入的设置
    let import_settings_content =
        fs::read_to_string(temp_settings_path).map_err(|e| format!("读取导入设置失败: {}", e))?;

    let import_settings: serde_json::Value = serde_json::from_str(&import_settings_content)
        .map_err(|e| format!("解析导入设置失败: {}", e))?;

    // 读取现有设置
    let settings_path = app_data_dir.join("settings.json");
    let mut current_settings = if settings_path.exists() {
        let content =
            fs::read_to_string(&settings_path).map_err(|e| format!("读取现有设置失败: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("解析现有设置失败: {}", e))?
    } else {
        serde_json::json!({})
    };

    // 合并设置（导入的设置会覆盖现有设置中的相同字段）
    if let (Some(current_obj), Some(import_obj)) = (
        current_settings.as_object_mut(),
        import_settings.as_object(),
    ) {
        for (key, value) in import_obj {
            current_obj.insert(key.clone(), value.clone());
        }
    }

    // 保存合并后的设置
    let merged_content = serde_json::to_string_pretty(&current_settings)
        .map_err(|e| format!("序列化合并设置失败: {}", e))?;

    fs::write(&settings_path, merged_content).map_err(|e| format!("保存合并设置失败: {}", e))?;

    Ok(())
}

// 合并图片数据
async fn merge_images_data(temp_images_dir: &Path, app_data_dir: &Path) -> Result<(), String> {
    let target_images_dir = app_data_dir.join("clipboard_images");

    // 确保目标目录及其子目录存在
    fs::create_dir_all(&target_images_dir).map_err(|e| format!("创建图片目录失败: {}", e))?;
    fs::create_dir_all(&target_images_dir.join("images"))
        .map_err(|e| format!("创建images子目录失败: {}", e))?;
    fs::create_dir_all(&target_images_dir.join("thumbnails"))
        .map_err(|e| format!("创建thumbnails子目录失败: {}", e))?;

    // 递归复制整个图片目录结构
    copy_dir_recursively(temp_images_dir, &target_images_dir)?;
    Ok(())
}

// 递归复制目录
fn copy_dir_recursively(source: &Path, target: &Path) -> Result<(), String> {
    let entries = fs::read_dir(source).map_err(|e| format!("读取源目录失败: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
        let source_path = entry.path();
        let file_name = entry.file_name();
        let target_path = target.join(&file_name);

        if source_path.is_dir() {
            // 如果是目录，递归复制
            fs::create_dir_all(&target_path).map_err(|e| format!("创建目标目录失败: {}", e))?;
            copy_dir_recursively(&source_path, &target_path)?;
        } else {
            // 如果是文件，复制文件（如果目标文件不存在）
            if !target_path.exists() {
                fs::copy(&source_path, &target_path).map_err(|e| format!("复制文件失败: {}", e))?;
            }
        }
    }

    Ok(())
}
