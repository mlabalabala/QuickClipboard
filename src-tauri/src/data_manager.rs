use chrono::Utc;
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
        export_time: Utc::now().to_rfc3339(),
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

    // 根据导入模式处理数据
    match options.mode {
        ImportMode::Replace => {
            // 替换模式：先备份现有数据，然后清空
            backup_current_data(&app_data_dir).await?;
            clear_current_data(&app_data_dir, &metadata.options).await?;
        }
        ImportMode::Merge => {
            // 合并模式：保留现有数据
            backup_current_data(&app_data_dir).await?;
        }
    }

    // 提取文件
    extract_files_from_zip(&mut archive, &app_data_dir, &metadata.options)?;

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
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
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
