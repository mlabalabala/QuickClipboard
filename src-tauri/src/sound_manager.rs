use dirs;
use once_cell::sync::Lazy;
use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

// 音效播放限流器 - 防止大量并发播放导致崩溃
static ACTIVE_SOUND_COUNT: AtomicUsize = AtomicUsize::new(0);
const MAX_CONCURRENT_SOUNDS: usize = 3; // 最大同时播放音效数量

// 音效播放计数器管理 - RAII模式确保计数器正确管理
struct SoundPlayGuard;

impl SoundPlayGuard {
    fn new() -> Option<Self> {
        let current = ACTIVE_SOUND_COUNT.load(Ordering::Relaxed);
        if current >= MAX_CONCURRENT_SOUNDS {
            // println!(
            //     "音效播放已达到最大并发数量 ({}), 跳过播放",
            //     MAX_CONCURRENT_SOUNDS
            // );
            return None;
        }

        ACTIVE_SOUND_COUNT.fetch_add(1, Ordering::Relaxed);
        Some(SoundPlayGuard)
    }
}

impl Drop for SoundPlayGuard {
    fn drop(&mut self) {
        ACTIVE_SOUND_COUNT.fetch_sub(1, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundSettings {
    pub enabled: bool,
    pub volume: f32,
    pub copy_sound_path: String,
    pub paste_sound_path: String,
    pub preset: String,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 0.5,
            copy_sound_path: String::new(),
            paste_sound_path: String::new(),
            preset: "classic".to_string(),
        }
    }
}

pub struct SoundManager {
    settings: Arc<Mutex<SoundSettings>>,
}

impl SoundManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            settings: Arc::new(Mutex::new(SoundSettings::default())),
        })
    }

    pub fn update_settings(&self, settings: SoundSettings) {
        if let Ok(mut current_settings) = self.settings.lock() {
            *current_settings = settings;
        }
    }

    pub fn get_settings(&self) -> SoundSettings {
        self.settings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn play_copy_sound(&self) {
        self.play_sound_async("copy");
    }

    pub fn play_paste_sound(&self) {
        self.play_sound_async("paste");
    }

    pub fn test_sound(
        &self,
        sound_path: &str,
        volume: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::play_sound_sync(sound_path, volume)
    }

    fn play_sound_async(&self, sound_type: &str) {
        let settings = self.get_settings();

        if !settings.enabled {
            return;
        }

        let sound_path = match sound_type {
            "copy" => &settings.copy_sound_path,
            "paste" => &settings.paste_sound_path,
            _ => return,
        };

        if sound_path.is_empty() {
            // 使用默认音效
            self.play_default_sound(sound_type, settings.volume);
            return;
        }

        let path = sound_path.clone();
        let volume = settings.volume;

        thread::spawn(move || {
            if let Err(e) = Self::play_sound_sync(&path, volume) {
                eprintln!("播放音效失败: {}", e);
            }
        });
    }

    pub fn play_sound_sync(
        sound_path: &str,
        volume: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 检查是否为网络URL
        if sound_path.starts_with("http://") || sound_path.starts_with("https://") {
            return Self::play_network_sound(sound_path, volume);
        }

        // 本地文件 - 支持相对路径
        let path = Self::resolve_sound_path(sound_path)?;
        if !path.exists() {
            return Err(format!("音效文件不存在: {}", path.display()).into());
        }

        Self::play_local_file(&path, volume)
    }

    // 解析音效文件路径，支持相对路径和Tauri资源文件
    fn resolve_sound_path(sound_path: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let path = Path::new(sound_path);

        // 如果是绝对路径，直接使用
        if path.is_absolute() {
            return Ok(path.to_path_buf());
        }

        // 相对路径：尝试多个可能的位置
        let mut possible_paths = Vec::new();

        // 1. 相对于当前工作目录
        possible_paths.push(std::env::current_dir()?.join(sound_path));

        // 2. 相对于可执行文件目录
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                possible_paths.push(exe_dir.join(sound_path));

                // 3. 相对于可执行文件目录的上级目录（开发模式）
                if let Some(parent_dir) = exe_dir.parent() {
                    possible_paths.push(parent_dir.join(sound_path));

                    // 4. 相对于项目根目录（开发模式下的src-tauri的上级目录）
                    if let Some(project_root) = parent_dir.parent() {
                        possible_paths.push(project_root.join(sound_path));
                    }
                }

                // 5. Tauri打包后的资源目录（Windows）
                // 在打包后，资源文件位于可执行文件同目录下的sounds文件夹
                possible_paths.push(exe_dir.join(sound_path));
            }
        }

        // 6. 尝试从环境变量获取项目根目录
        if let Ok(cargo_manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            let manifest_dir = Path::new(&cargo_manifest_dir);
            if let Some(project_root) = manifest_dir.parent() {
                possible_paths.push(project_root.join(sound_path));
            }
        }

        // 查找第一个存在的文件
        for candidate_path in &possible_paths {
            if candidate_path.exists() {
                return Ok(candidate_path.clone());
            }
        }

        // 如果都不存在，返回相对于当前目录的路径
        Ok(possible_paths[0].clone())
    }

    fn play_network_sound(url: &str, volume: f32) -> Result<(), Box<dyn std::error::Error>> {
        // 检查缓存
        let cache_path = Self::get_cached_sound(url)?;

        if cache_path.exists() {
            // 使用缓存文件
            println!("使用缓存音效: {:?}", cache_path);
            return Self::play_local_file(&cache_path, volume);
        }

        // 下载并缓存
        println!("下载网络音效: {}", url);
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async {
            let response = reqwest::get(url).await?;
            let bytes = response.bytes().await?;

            // 保存到缓存
            let mut file = std::fs::File::create(&cache_path)?;
            file.write_all(&bytes)?;

            // 更新缓存记录
            if let Ok(mut cache) = SOUND_CACHE.lock() {
                cache.insert(url.to_string(), cache_path.clone());
            }

            println!("音效已缓存到: {:?}", cache_path);

            Ok::<(), Box<dyn std::error::Error>>(())
        })?;

        // 播放缓存的文件
        Self::play_local_file(&cache_path, volume)
    }

    // 获取缓存音效路径
    fn get_cached_sound(url: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        // 先检查内存缓存
        if let Ok(cache) = SOUND_CACHE.lock() {
            if let Some(path) = cache.get(url) {
                if path.exists() {
                    return Ok(path.clone());
                }
            }
        }

        // 生成缓存路径
        let cache_dir = get_cache_dir()?;
        let filename = get_cache_filename(url);
        let cache_path = cache_dir.join(filename);

        Ok(cache_path)
    }

    // 播放本地文件 - 优化内存管理和错误处理
    fn play_local_file(path: &Path, volume: f32) -> Result<(), Box<dyn std::error::Error>> {
        // 使用作用域确保资源正确释放
        let result = {
            let (_stream, stream_handle) = OutputStream::try_default()?;
            let sink = Sink::try_new(&stream_handle)?;

            let file = File::open(path)?;
            let source = Decoder::new(BufReader::new(file))?;

            sink.set_volume(volume);
            sink.append(source);

            // 使用更短的超时时间，避免长时间阻塞
            let timeout = std::time::Duration::from_secs(3);
            let start_time = std::time::Instant::now();

            while !sink.empty() && start_time.elapsed() < timeout {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }

            // 确保sink停止播放
            sink.stop();
            Ok(())
        };

        // 强制垃圾回收，释放内存
        std::thread::sleep(std::time::Duration::from_millis(1));
        result
    }

    fn play_default_sound(&self, sound_type: &str, volume: f32) {
        // 播放系统默认音效
        let sound_type = sound_type.to_string(); // 转换为拥有的字符串
        thread::spawn(move || {
            let frequency = match sound_type.as_str() {
                "copy" => 800.0,  // 复制音效：较高频率
                "paste" => 600.0, // 粘贴音效：较低频率
                _ => 700.0,
            };

            if let Err(e) = Self::play_beep(frequency, 100, volume) {
                eprintln!("播放默认音效失败: {}", e);
            }
        });
    }

    pub fn play_beep(
        frequency: f32,
        duration_ms: u64,
        volume: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 使用作用域确保资源正确释放
        let result = {
            let (_stream, stream_handle) = OutputStream::try_default()?;
            let sink = Sink::try_new(&stream_handle)?;

            // 生成简单的正弦波音效
            let sample_rate = 44100;
            let duration_samples = (sample_rate as f32 * duration_ms as f32 / 1000.0) as usize;

            let mut samples = Vec::with_capacity(duration_samples);
            for i in 0..duration_samples {
                let t = i as f32 / sample_rate as f32;
                let sample = (2.0 * std::f32::consts::PI * frequency * t).sin();
                samples.push(sample);
            }

            let source = rodio::buffer::SamplesBuffer::new(1, sample_rate, samples);

            sink.set_volume(volume);
            sink.append(source);

            // 使用更短的超时时间
            let timeout = std::time::Duration::from_millis(duration_ms + 500); // 音效时长 + 0.5秒缓冲
            let start_time = std::time::Instant::now();

            while !sink.empty() && start_time.elapsed() < timeout {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }

            // 确保sink停止播放
            sink.stop();
            Ok(())
        };

        // 强制垃圾回收，释放内存
        std::thread::sleep(std::time::Duration::from_millis(1));
        result
    }
}

// 全局音效设置和缓存
static GLOBAL_SOUND_SETTINGS: Lazy<Arc<Mutex<SoundSettings>>> =
    Lazy::new(|| Arc::new(Mutex::new(SoundSettings::default())));
static SOUND_CACHE: Lazy<Arc<Mutex<HashMap<String, PathBuf>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// 获取缓存目录
fn get_cache_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let cache_dir = dirs::cache_dir()
        .ok_or("无法获取缓存目录")?
        .join("QuickClipboard")
        .join("sounds");

    // 确保缓存目录存在
    if !cache_dir.exists() {
        create_dir_all(&cache_dir)?;
    }

    Ok(cache_dir)
}

// 生成缓存文件名
fn get_cache_filename(url: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = hasher.finish();

    // 尝试从URL获取文件扩展名
    let extension = url
        .split('.')
        .last()
        .and_then(|ext| {
            let ext = ext.split('?').next().unwrap_or(ext); // 移除查询参数
            if ["wav", "mp3", "ogg", "flac", "m4a", "aac"].contains(&ext.to_lowercase().as_str()) {
                Some(ext)
            } else {
                None
            }
        })
        .unwrap_or("wav");

    format!("{}.{}", hash, extension)
}

pub fn initialize_sound_manager() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化全局设置
    let _settings = GLOBAL_SOUND_SETTINGS.lock().unwrap();

    // 确保缓存目录存在
    let _cache_dir = get_cache_dir()?;

    println!("音效管理器初始化成功");
    Ok(())
}

pub fn update_sound_settings(settings: SoundSettings) {
    if let Ok(mut global_settings) = GLOBAL_SOUND_SETTINGS.lock() {
        *global_settings = settings;
    }
}

pub fn play_copy_sound() {
    if let Ok(settings) = GLOBAL_SOUND_SETTINGS.lock() {
        if settings.enabled {
            let sound_path = settings.copy_sound_path.clone();
            let volume = settings.volume;

            thread::spawn(move || {
                // 使用限流器防止过多并发播放
                let _guard = match SoundPlayGuard::new() {
                    Some(guard) => guard,
                    None => return, // 达到最大并发数，跳过播放
                };

                let effective_path = if sound_path.is_empty() {
                    // 使用默认复制音效文件
                    "sounds/copy.mp3".to_string()
                } else {
                    sound_path
                };

                // 播放音效文件
                if let Err(e) = SoundManager::play_sound_sync(&effective_path, volume) {
                    eprintln!("播放复制音效失败: {}", e);
                    // 如果文件播放失败，回退到代码生成的音效
                    if let Err(e2) = SoundManager::play_beep(800.0, 100, volume) {
                        eprintln!("播放默认复制音效也失败: {}", e2);
                    }
                }
                // _guard 在这里自动释放，计数器减1
            });
        }
    }
}

pub fn play_paste_sound() {
    if let Ok(settings) = GLOBAL_SOUND_SETTINGS.lock() {
        if settings.enabled {
            let sound_path = settings.paste_sound_path.clone();
            let volume = settings.volume;

            thread::spawn(move || {
                // 使用限流器防止过多并发播放
                let _guard = match SoundPlayGuard::new() {
                    Some(guard) => guard,
                    None => return, // 达到最大并发数，跳过播放
                };

                let effective_path = if sound_path.is_empty() {
                    // 使用默认粘贴音效文件
                    "sounds/paste.mp3".to_string()
                } else {
                    sound_path
                };

                // 播放音效文件
                if let Err(e) = SoundManager::play_sound_sync(&effective_path, volume) {
                    eprintln!("播放粘贴音效失败: {}", e);
                    // 如果文件播放失败，回退到代码生成的音效
                    if let Err(e2) = SoundManager::play_beep(600.0, 100, volume) {
                        eprintln!("播放默认粘贴音效也失败: {}", e2);
                    }
                }
                // _guard 在这里自动释放，计数器减1
            });
        }
    }
}

pub fn play_scroll_sound() {
    // 检查预览滚动音效是否启用
    let app_settings = crate::settings::get_global_settings();
    if !app_settings.preview_scroll_sound {
        return; // 滚动音效已禁用
    }

    if let Ok(settings) = GLOBAL_SOUND_SETTINGS.lock() {
        if settings.enabled {
            let sound_path = app_settings.preview_scroll_sound_path.clone();
            let volume = settings.volume;

            thread::spawn(move || {
                // 使用限流器防止过多并发播放
                let _guard = match SoundPlayGuard::new() {
                    Some(guard) => guard,
                    None => return, // 达到最大并发数，跳过播放
                };

                let effective_path = if sound_path.is_empty() {
                    // 使用默认滚动音效文件
                    "sounds/roll.mp3".to_string()
                } else {
                    sound_path
                };

                // 播放音效文件
                if let Err(e) = SoundManager::play_sound_sync(&effective_path, volume) {
                    eprintln!("播放滚动音效失败: {}", e);
                    // 如果文件播放失败，回退到代码生成的音效
                    if let Err(e2) = SoundManager::play_beep(400.0, 50, volume) {
                        eprintln!("播放默认滚动音效也失败: {}", e2);
                    }
                }
                // _guard 在这里自动释放，计数器减1
            });
        }
    }
}

// 清理音效缓存
pub fn clear_sound_cache() -> Result<(), String> {
    // 清理内存缓存
    if let Ok(mut cache) = SOUND_CACHE.lock() {
        cache.clear();
    }

    // 清理文件缓存
    if let Ok(cache_dir) = get_cache_dir() {
        if cache_dir.exists() {
            std::fs::remove_dir_all(&cache_dir).map_err(|e| format!("清理缓存目录失败: {}", e))?;

            // 重新创建缓存目录
            create_dir_all(&cache_dir).map_err(|e| format!("重新创建缓存目录失败: {}", e))?;
        }
    }

    println!("音效缓存已清理");
    Ok(())
}

// 获取当前活跃音效播放数量
pub fn get_active_sound_count() -> usize {
    ACTIVE_SOUND_COUNT.load(Ordering::Relaxed)
}

// 等待所有音效播放完成（用于应用退出时）
pub fn wait_for_sounds_to_finish(timeout_ms: u64) {
    let start_time = std::time::Instant::now();
    let timeout = std::time::Duration::from_millis(timeout_ms);

    while ACTIVE_SOUND_COUNT.load(Ordering::Relaxed) > 0 && start_time.elapsed() < timeout {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let remaining = ACTIVE_SOUND_COUNT.load(Ordering::Relaxed);
    if remaining > 0 {
        println!("等待音效播放完成超时，仍有 {} 个音效在播放", remaining);
    }
}
