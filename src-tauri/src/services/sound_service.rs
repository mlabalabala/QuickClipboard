/// 音效服务 - 处理音效播放和管理相关的业务逻辑
pub struct SoundService;

impl SoundService {
    /// 测试音效播放
    pub async fn test_sound(sound_path: String, volume: f32, sound_type: Option<String>) -> Result<(), String> {
        let volume_normalized = volume / 100.0; // 将0-100转换为0.0-1.0

        // 在后台线程中播放音效，避免阻塞前端
        let sound_path_clone = sound_path.clone();
        let sound_type_clone = sound_type.clone();
        
        tokio::spawn(async move {
            // 检查文件是否存在
            let effective_path = if std::path::Path::new(&sound_path_clone).exists() {
                sound_path_clone
            } else {
                // 如果文件不存在，使用应用数据目录下的音效文件
                match crate::data_manager::get_app_data_dir() {
                    Ok(app_data_dir) => {
                        let sound_file_path = app_data_dir.join(&sound_path_clone);
                        sound_file_path.to_string_lossy().to_string()
                    }
                    Err(_) => sound_path_clone,
                }
            };

            // 尝试播放指定的音效文件
            if let Err(e) = 
                crate::sound_manager::SoundManager::play_sound_sync(&effective_path, volume_normalized)
            {
                eprintln!("测试音效失败: {}", e);
                // 如果文件播放失败，回退到代码生成的音效
                
                // 根据音效类型生成不同频率的提示音
                let frequency = match sound_type_clone.as_deref() {
                    Some("copy") => 800,   // 复制音效：中频
                    Some("paste") => 600,  // 粘贴音效：低频
                    _ => 1000,             // 默认：高频
                };
                
                if let Err(e2) = 
                    crate::sound_manager::SoundManager::play_beep(frequency as f32, 200, volume_normalized)
                {
                    eprintln!("测试默认音效也失败: {}", e2);
                }
            }
        });
        
        Ok(())
    }

    /// 播放粘贴音效
    pub fn play_paste_sound() -> Result<(), String> {
        crate::sound_manager::play_paste_sound();
        Ok(())
    }

    /// 播放滚动音效
    pub fn play_scroll_sound() -> Result<(), String> {
        crate::sound_manager::play_scroll_sound();
        Ok(())
    }

    /// 清理音效缓存
    pub fn clear_sound_cache() -> Result<(), String> {
        crate::sound_manager::clear_sound_cache()
    }

    /// 获取当前活跃音效播放数量
    pub fn get_active_sound_count() -> usize {
        crate::sound_manager::get_active_sound_count()
    }

    /// 获取音效状态信息
    pub fn get_sound_status() -> Result<serde_json::Value, String> {
        let active_count = crate::sound_manager::get_active_sound_count();
        Ok(serde_json::json!({
            "active_sounds": active_count,
            "max_concurrent": 3
        }))
    }
}
