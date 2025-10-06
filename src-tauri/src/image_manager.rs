use base64::{engine::general_purpose as b64_engine, Engine as _};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

// 图片存储配置
const IMAGES_DIR: &str = "clipboard_images";

/// 保存图片文件，前端直接访问
pub struct ImageManager {
    images_dir: PathBuf,
}

impl ImageManager {
    pub fn new() -> Result<Self, String> {
        let app_data_dir = get_app_data_dir()?;
        let images_dir = app_data_dir.join(IMAGES_DIR);

        // 创建图片目录
        fs::create_dir_all(&images_dir)
            .map_err(|e| format!("创建图片目录失败: {}", e))?;

        Ok(ImageManager { images_dir })
    }

    /// 保存图片并返回图片ID
    /// 使用内容的SHA256哈希前16位作为图片ID（自动去重）
    pub fn save_image(&self, data_url: &str) -> Result<String, String> {
        // 解析data URL获取图片数据
        let image_data = self.parse_data_url(data_url)?;

        // 计算内容hash作为唯一ID（自动去重）
        let mut hasher = Sha256::new();
        hasher.update(&image_data);
        let hash = format!("{:x}", hasher.finalize());
        let image_id = hash[..16].to_string();

        // 构建文件路径
        let file_path = self.images_dir.join(format!("{}.png", image_id));

        // 如果文件已存在，直接返回ID（自动去重）
        if file_path.exists() {
            return Ok(image_id);
        }

        // 保存图片为PNG格式
        let img = image::load_from_memory(&image_data)
            .map_err(|e| format!("解析图片失败: {}", e))?;
        
        img.save_with_format(&file_path, image::ImageFormat::Png)
            .map_err(|e| format!("保存图片失败: {}", e))?;

        Ok(image_id)
    }

    /// 获取图片文件路径
    pub fn get_image_file_path(&self, image_id: &str) -> Result<String, String> {
        let file_path = self.images_dir.join(format!("{}.png", image_id));
        
        if !file_path.exists() {
            return Err(format!("图片文件不存在: {}", image_id));
        }
        
        Ok(file_path.to_string_lossy().to_string())
    }

    /// 获取图片数据URL（用于粘贴到其他应用）
    pub fn get_image_data_url(&self, image_id: &str) -> Result<String, String> {
        let file_path = self.images_dir.join(format!("{}.png", image_id));

        if !file_path.exists() {
            return Err(format!("图片文件不存在: {}", image_id));
        }

        let image_data = fs::read(&file_path)
            .map_err(|e| format!("读取图片文件失败: {}", e))?;

        let base64_string = b64_engine::STANDARD.encode(&image_data);
        Ok(format!("data:image/png;base64,{}", base64_string))
    }

    /// 删除图片
    pub fn delete_image(&self, image_id: &str) -> Result<(), String> {
        let file_path = self.images_dir.join(format!("{}.png", image_id));

        if file_path.exists() {
            fs::remove_file(&file_path)
                .map_err(|e| format!("删除图片失败: {}", e))?;
        }

        Ok(())
    }

    /// 清理未使用的图片
    pub fn cleanup_unused_images(&self, used_image_ids: &[String]) -> Result<(), String> {
        let entries = fs::read_dir(&self.images_dir)
            .map_err(|e| format!("读取图片目录失败: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "png") {
                if let Some(file_stem) = path.file_stem() {
                    let image_id = file_stem.to_string_lossy().to_string();
                    if !used_image_ids.contains(&image_id) {
                        let _ = self.delete_image(&image_id);
                    }
                }
            }
        }

        Ok(())
    }

    /// 解析data URL，提取图片二进制数据
    fn parse_data_url(&self, data_url: &str) -> Result<Vec<u8>, String> {
        if !data_url.starts_with("data:image/") {
            return Err("不是有效的图片data URL".to_string());
        }

        let comma_pos = data_url
            .find(',')
            .ok_or_else(|| "无效的data URL格式".to_string())?;

        let encoded = &data_url[(comma_pos + 1)..];

        b64_engine::STANDARD
            .decode(encoded)
            .map_err(|e| format!("Base64解码失败: {}", e))
    }
}

/// 获取应用数据目录
fn get_app_data_dir() -> Result<PathBuf, String> {
    crate::settings::get_data_directory()
}

/// 全局图片管理器实例
use once_cell::sync::Lazy;
use std::sync::Mutex;

static IMAGE_MANAGER: Lazy<Result<Mutex<ImageManager>, String>> =
    Lazy::new(|| ImageManager::new().map(Mutex::new));

pub fn get_image_manager() -> Result<&'static Mutex<ImageManager>, String> {
    IMAGE_MANAGER.as_ref().map_err(|e| e.clone())
}
