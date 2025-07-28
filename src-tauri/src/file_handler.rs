// 文件处理模块 - 处理文件复制、图标获取等功能

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub is_directory: bool,
    pub icon_data: Option<String>, // Base64编码的图标数据
    pub file_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileClipboardData {
    pub files: Vec<FileInfo>,
    pub operation: String, // "copy" 或 "cut"
}

// 将文件路径写入剪贴板
#[cfg(windows)]
pub fn set_clipboard_files(file_paths: &[String]) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::{HANDLE, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::System::Ole::CF_HDROP;

    unsafe {
        // 打开剪贴板
        if OpenClipboard(HWND(0)).is_err() {
            return Err("无法打开剪贴板".to_string());
        }

        // 清空剪贴板
        if EmptyClipboard().is_err() {
            let _ = CloseClipboard();
            return Err("无法清空剪贴板".to_string());
        }

        // 计算所需内存大小
        let mut total_size = std::mem::size_of::<windows::Win32::UI::Shell::DROPFILES>();
        for path in file_paths {
            let wide_path: Vec<u16> = OsStr::new(path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            total_size += wide_path.len() * 2; // UTF-16 字符
        }
        total_size += 2; // 双重空终止符

        // 分配全局内存
        let hmem = match GlobalAlloc(GMEM_MOVEABLE, total_size) {
            Ok(h) => h,
            Err(_) => {
                let _ = CloseClipboard();
                return Err("无法分配内存".to_string());
            }
        };

        if hmem.is_invalid() {
            let _ = CloseClipboard();
            return Err("无法分配内存".to_string());
        }

        let ptr = GlobalLock(hmem);
        if ptr.is_null() {
            let _ = CloseClipboard();
            return Err("无法锁定内存".to_string());
        }

        // 设置 DROPFILES 结构
        let dropfiles = ptr as *mut windows::Win32::UI::Shell::DROPFILES;
        (*dropfiles).pFiles = std::mem::size_of::<windows::Win32::UI::Shell::DROPFILES>() as u32;
        (*dropfiles).pt.x = 0;
        (*dropfiles).pt.y = 0;
        (*dropfiles).fNC = windows::Win32::Foundation::BOOL(0);
        (*dropfiles).fWide = windows::Win32::Foundation::BOOL(1); // Unicode

        // 写入文件路径
        let mut offset = std::mem::size_of::<windows::Win32::UI::Shell::DROPFILES>();
        for path in file_paths {
            let wide_path: Vec<u16> = OsStr::new(path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let dest_ptr = (ptr as *mut u8).add(offset) as *mut u16;
            std::ptr::copy_nonoverlapping(wide_path.as_ptr(), dest_ptr, wide_path.len());
            offset += wide_path.len() * 2;
        }

        // 添加双重空终止符
        let final_ptr = (ptr as *mut u8).add(offset) as *mut u16;
        *final_ptr = 0;

        let _ = GlobalUnlock(hmem);

        // 设置剪贴板数据
        if SetClipboardData(CF_HDROP.0 as u32, HANDLE(hmem.0 as isize)).is_err() {
            let _ = CloseClipboard();
            return Err("无法设置剪贴板数据".to_string());
        }

        let _ = CloseClipboard();
        Ok(())
    }
}

#[cfg(not(windows))]
pub fn set_clipboard_files(_file_paths: &[String]) -> Result<(), String> {
    Err("当前平台不支持文件剪贴板操作".to_string())
}

// 从剪贴板获取文件路径列表
#[cfg(windows)]
pub fn get_clipboard_files() -> Result<Vec<String>, String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    };
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    use windows::Win32::System::Ole::CF_HDROP;
    use windows::Win32::UI::Shell::DragQueryFileW;

    unsafe {
        // 打开剪贴板
        if OpenClipboard(HWND(0)).is_err() {
            return Err("无法打开剪贴板".to_string());
        }

        let mut files = Vec::new();

        // 检查是否有文件格式
        if IsClipboardFormatAvailable(CF_HDROP.0 as u32).is_ok() {
            // 获取文件数据
            if let Ok(hdrop) = GetClipboardData(CF_HDROP.0 as u32) {
                if !hdrop.is_invalid() {
                    let hdrop_ptr = GlobalLock(windows::Win32::Foundation::HGLOBAL(
                        hdrop.0 as *mut core::ffi::c_void,
                    ));
                    if !hdrop_ptr.is_null() {
                        // 获取文件数量
                        let file_count = DragQueryFileW(
                            windows::Win32::UI::Shell::HDROP(hdrop_ptr as isize),
                            0xFFFFFFFF,
                            None,
                        );

                        for i in 0..file_count {
                            // 获取文件路径长度
                            let path_len = DragQueryFileW(
                                windows::Win32::UI::Shell::HDROP(hdrop_ptr as isize),
                                i,
                                None,
                            );
                            if path_len > 0 {
                                // 获取文件路径
                                let mut buffer = vec![0u16; (path_len + 1) as usize];
                                let actual_len = DragQueryFileW(
                                    windows::Win32::UI::Shell::HDROP(hdrop_ptr as isize),
                                    i,
                                    Some(buffer.as_mut_slice()),
                                );

                                if actual_len > 0 {
                                    buffer.truncate(actual_len as usize);
                                    let os_string = OsString::from_wide(&buffer);
                                    if let Some(path_str) = os_string.to_str() {
                                        files.push(path_str.to_string());
                                    }
                                }
                            }
                        }

                        let _ = GlobalUnlock(windows::Win32::Foundation::HGLOBAL(
                            hdrop.0 as *mut core::ffi::c_void,
                        ));
                    }
                }
            }
        }

        let _ = CloseClipboard();
        Ok(files)
    }
}

#[cfg(not(windows))]
pub fn get_clipboard_files() -> Result<Vec<String>, String> {
    // 非Windows平台暂不支持
    Err("当前平台不支持文件剪贴板操作".to_string())
}

// 获取文件信息
pub fn get_file_info(path: &str) -> Result<FileInfo, String> {
    let path_buf = PathBuf::from(path);

    if !path_buf.exists() {
        return Err(format!("文件不存在: {}", path));
    }

    let metadata = fs::metadata(&path_buf).map_err(|e| format!("获取文件元数据失败: {}", e))?;

    let name = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("未知文件")
        .to_string();

    let is_directory = metadata.is_dir();
    let size = metadata.len();

    // 获取文件类型
    let file_type = if is_directory {
        "文件夹".to_string()
    } else {
        path_buf
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_uppercase())
            .unwrap_or_else(|| "文件".to_string())
    };

    // 获取文件图标
    let icon_data = get_file_icon(path)?;

    Ok(FileInfo {
        path: path.to_string(),
        name,
        size,
        is_directory,
        icon_data: Some(icon_data),
        file_type,
    })
}

// 获取文件图标（简化版本）
#[cfg(windows)]
pub fn get_file_icon(path: &str) -> Result<String, String> {
    use std::path::Path;

    let path_obj = Path::new(path);

    // 根据文件类型返回不同的图标
    if path_obj.is_dir() {
        // 文件夹图标
        Ok(get_folder_icon())
    } else {
        // 根据文件扩展名返回图标
        let extension = path_obj
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        Ok(get_file_icon_by_extension(&extension))
    }
}

#[cfg(not(windows))]
pub fn get_file_icon(_path: &str) -> Result<String, String> {
    // 非Windows平台返回默认图标
    Ok(get_default_file_icon())
}

// 获取文件夹图标
fn get_folder_icon() -> String {
    // 简单的文件夹图标 SVG，转换为 base64
    "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIHZpZXdCb3g9IjAgMCAxNiAxNiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHBhdGggZD0iTTIgM0gyVjEzSDEzVjVINy41TDYgM0gyWiIgZmlsbD0iIzU0OUJGRiIvPgo8L3N2Zz4K".to_string()
}

// 根据文件扩展名获取图标
fn get_file_icon_by_extension(extension: &str) -> String {
    match extension {
        "txt" | "md" | "log" => get_text_file_icon(),
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" => get_image_file_icon(),
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" => get_video_file_icon(),
        "mp3" | "wav" | "flac" | "aac" | "ogg" => get_audio_file_icon(),
        "pdf" => get_pdf_file_icon(),
        "doc" | "docx" => get_word_file_icon(),
        "xls" | "xlsx" => get_excel_file_icon(),
        "ppt" | "pptx" => get_powerpoint_file_icon(),
        "zip" | "rar" | "7z" | "tar" | "gz" => get_archive_file_icon(),
        "exe" | "msi" => get_executable_file_icon(),
        _ => get_default_file_icon(),
    }
}

// 各种文件类型的图标
fn get_text_file_icon() -> String {
    "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIHZpZXdCb3g9IjAgMCAxNiAxNiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHBhdGggZD0iTTMgMkgxMFYxNEgzVjJaIiBmaWxsPSIjRkZGRkZGIiBzdHJva2U9IiM5OTk5OTkiLz4KPHA+PC9wPgo8L3N2Zz4K".to_string()
}

fn get_image_file_icon() -> String {
    "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIHZpZXdCb3g9IjAgMCAxNiAxNiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3QgeD0iMiIgeT0iMiIgd2lkdGg9IjEyIiBoZWlnaHQ9IjEyIiBmaWxsPSIjRkY2QjZCIi8+Cjwvc3ZnPgo=".to_string()
}

fn get_video_file_icon() -> String {
    "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIHZpZXdCb3g9IjAgMCAxNiAxNiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3QgeD0iMiIgeT0iMiIgd2lkdGg9IjEyIiBoZWlnaHQ9IjEyIiBmaWxsPSIjRkY5NTAwIi8+Cjwvc3ZnPgo=".to_string()
}

fn get_audio_file_icon() -> String {
    "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIHZpZXdCb3g9IjAgMCAxNiAxNiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3QgeD0iMiIgeT0iMiIgd2lkdGg9IjEyIiBoZWlnaHQ9IjEyIiBmaWxsPSIjOUMzNUZGIi8+Cjwvc3ZnPgo=".to_string()
}

fn get_pdf_file_icon() -> String {
    "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIHZpZXdCb3g9IjAgMCAxNiAxNiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3QgeD0iMiIgeT0iMiIgd2lkdGg9IjEyIiBoZWlnaHQ9IjEyIiBmaWxsPSIjRkYwMDAwIi8+Cjwvc3ZnPgo=".to_string()
}

fn get_word_file_icon() -> String {
    "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIHZpZXdCb3g9IjAgMCAxNiAxNiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3QgeD0iMiIgeT0iMiIgd2lkdGg9IjEyIiBoZWlnaHQ9IjEyIiBmaWxsPSIjMjk3NEZGIi8+Cjwvc3ZnPgo=".to_string()
}

fn get_excel_file_icon() -> String {
    "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIHZpZXdCb3g9IjAgMCAxNiAxNiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3QgeD0iMiIgeT0iMiIgd2lkdGg9IjEyIiBoZWlnaHQ9IjEyIiBmaWxsPSIjMDBCMDUwIi8+Cjwvc3ZnPgo=".to_string()
}

fn get_powerpoint_file_icon() -> String {
    "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIHZpZXdCb3g9IjAgMCAxNiAxNiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3QgeD0iMiIgeT0iMiIgd2lkdGg9IjEyIiBoZWlnaHQ9IjEyIiBmaWxsPSIjRkY0NTAwIi8+Cjwvc3ZnPgo=".to_string()
}

fn get_archive_file_icon() -> String {
    "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIHZpZXdCb3g9IjAgMCAxNiAxNiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3QgeD0iMiIgeT0iMiIgd2lkdGg9IjEyIiBoZWlnaHQ9IjEyIiBmaWxsPSIjRkZEQjAwIi8+Cjwvc3ZnPgo=".to_string()
}

fn get_executable_file_icon() -> String {
    "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIHZpZXdCb3g9IjAgMCAxNiAxNiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3QgeD0iMiIgeT0iMiIgd2lkdGg9IjEyIiBoZWlnaHQ9IjEyIiBmaWxsPSIjNjY2NjY2Ii8+Cjwvc3ZnPgo=".to_string()
}

fn get_default_file_icon() -> String {
    "data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMTYiIGhlaWdodD0iMTYiIHZpZXdCb3g9IjAgMCAxNiAxNiIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPHJlY3QgeD0iMiIgeT0iMiIgd2lkdGg9IjEyIiBoZWlnaHQ9IjEyIiBmaWxsPSIjQ0NDQ0NDIi8+Cjwvc3ZnPgo=".to_string()
}

// 复制文件到目标位置
pub fn copy_files_to_target(files: &[String], target_dir: &str) -> Result<Vec<String>, String> {
    let target_path = Path::new(target_dir);

    if !target_path.exists() {
        return Err(format!("目标目录不存在: {}", target_dir));
    }

    if !target_path.is_dir() {
        return Err(format!("目标路径不是目录: {}", target_dir));
    }

    let mut copied_files = Vec::new();

    for file_path in files {
        let source_path = Path::new(file_path);
        if !source_path.exists() {
            continue; // 跳过不存在的文件
        }

        let file_name = source_path
            .file_name()
            .ok_or_else(|| format!("无法获取文件名: {}", file_path))?;

        let target_file_path = target_path.join(file_name);

        // 如果目标文件已存在，生成新名称
        let final_target_path = generate_unique_path(&target_file_path)?;

        if source_path.is_dir() {
            // 复制目录
            copy_dir_recursive(source_path, &final_target_path)?;
        } else {
            // 复制文件
            fs::copy(source_path, &final_target_path)
                .map_err(|e| format!("复制文件失败 {}: {}", file_path, e))?;
        }

        copied_files.push(final_target_path.to_string_lossy().to_string());
    }

    Ok(copied_files)
}

// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("创建目录失败 {}: {}", dst.display(), e))?;

    for entry in fs::read_dir(src).map_err(|e| format!("读取目录失败 {}: {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| format!("读取目录项失败: {}", e))?;

        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("复制文件失败 {}: {}", src_path.display(), e))?;
        }
    }

    Ok(())
}

// 生成唯一的文件路径（如果文件已存在，添加数字后缀）
fn generate_unique_path(path: &Path) -> Result<PathBuf, String> {
    if !path.exists() {
        return Ok(path.to_path_buf());
    }

    let parent = path.parent().unwrap_or(Path::new(""));
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    for i in 1..1000 {
        let new_name = if extension.is_empty() {
            format!("{} ({})", stem, i)
        } else {
            format!("{} ({}).{}", stem, i, extension)
        };

        let new_path = parent.join(new_name);
        if !new_path.exists() {
            return Ok(new_path);
        }
    }

    Err("无法生成唯一文件名".to_string())
}
