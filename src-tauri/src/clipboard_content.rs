use crate::clipboard_history;
use arboard::Clipboard;
use base64::{engine::general_purpose as b64_engine, Engine as _};
use image::{ImageBuffer, ImageOutputFormat, Rgba};
use std::borrow::Cow;

// Windows CF_DIB 常量（0x0008）
#[cfg(windows)]
const CF_DIB: u32 = 8;

// === 图像与 DataURL 转换辅助函数 ===
pub fn image_to_data_url(image: &arboard::ImageData) -> String {
    let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(
        image.width as u32,
        image.height as u32,
        image.bytes.clone().into_owned(),
    )
    .expect("无法创建图像缓冲区");
    let mut png_bytes: Vec<u8> = Vec::new();
    let _ = image::DynamicImage::ImageRgba8(buffer).write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        ImageOutputFormat::Png,
    );
    let b64 = b64_engine::STANDARD.encode(png_bytes);
    format!("data:image/png;base64,{}", b64)
}

pub fn data_url_to_bgra_and_png(data_url: &str) -> Result<(Vec<u8>, Vec<u8>, u32, u32), String> {
    let comma = data_url
        .find(',')
        .ok_or_else(|| "无效Data URL".to_string())?;
    let encoded = &data_url[(comma + 1)..];
    let png_bytes = b64_engine::STANDARD
        .decode(encoded)
        .map_err(|e| format!("Base64解码失败: {}", e))?;

    let img = image::load_from_memory(&png_bytes)
        .map_err(|e| format!("解析PNG失败: {}", e))?
        .to_rgba8();
    let (width, height) = img.dimensions();
    let mut bgra: Vec<u8> = Vec::with_capacity((width * height * 4) as usize);
    for px in img.pixels() {
        let [r, g, b, a] = px.0;
        bgra.extend_from_slice(&[b, g, r, a]);
    }
    Ok((bgra, png_bytes, width, height))
}

#[cfg(windows)]
pub fn set_windows_clipboard_image(
    bgra: &[u8],
    png_bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<(), String> {
    use windows::core::w;
    use windows::Win32::Foundation::{HANDLE, HGLOBAL, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};

    unsafe {
        if OpenClipboard(HWND(0)).is_err() {
            return Err("打开剪贴板失败".into());
        }
        EmptyClipboard();

        // ---------- 写入 CF_DIB ----------
        let mut dib: Vec<u8> = Vec::with_capacity(40 + bgra.len());
        dib.extend_from_slice(&(40u32).to_le_bytes()); // biSize
        dib.extend_from_slice(&(width as i32).to_le_bytes()); // biWidth
        dib.extend_from_slice(&(-(height as i32)).to_le_bytes()); // biHeight (负值 = top-down)
        dib.extend_from_slice(&(1u16).to_le_bytes()); // biPlanes
        dib.extend_from_slice(&(32u16).to_le_bytes()); // biBitCount
        dib.extend_from_slice(&(0u32).to_le_bytes()); // biCompression = BI_RGB
        dib.extend_from_slice(&(0u32).to_le_bytes()); // biSizeImage
        dib.extend_from_slice(&(0i32).to_le_bytes()); // biXPelsPerMeter
        dib.extend_from_slice(&(0i32).to_le_bytes()); // biYPelsPerMeter
        dib.extend_from_slice(&(0u32).to_le_bytes()); // biClrUsed
        dib.extend_from_slice(&(0u32).to_le_bytes()); // biClrImportant
        dib.extend_from_slice(bgra);

        let hmem_dib: HGLOBAL =
            GlobalAlloc(GMEM_MOVEABLE, dib.len()).map_err(|e| format!("GlobalAlloc失败: {e}"))?;
        if !hmem_dib.0.is_null() {
            let ptr = GlobalLock(hmem_dib) as *mut u8;
            if !ptr.is_null() {
                std::ptr::copy_nonoverlapping(dib.as_ptr(), ptr, dib.len());
                GlobalUnlock(hmem_dib);
                SetClipboardData(CF_DIB, HANDLE(hmem_dib.0 as isize));
            }
        }

        // ---------- 写入 PNG 自定义格式 ----------
        let fmt_png = RegisterClipboardFormatW(w!("PNG"));
        if fmt_png != 0 {
            let hmem_png: HGLOBAL = GlobalAlloc(GMEM_MOVEABLE, png_bytes.len())
                .map_err(|e| format!("GlobalAlloc失败: {e}"))?;
            if !hmem_png.0.is_null() {
                let ptr = GlobalLock(hmem_png) as *mut u8;
                if !ptr.is_null() {
                    std::ptr::copy_nonoverlapping(png_bytes.as_ptr(), ptr, png_bytes.len());
                    GlobalUnlock(hmem_png);
                    SetClipboardData(fmt_png, HANDLE(hmem_png.0 as isize));
                }
            }
        }

        CloseClipboard();
    }
    Ok(())
}

pub fn data_url_to_image(data_url: &str) -> Result<arboard::ImageData<'static>, String> {
    let comma = data_url
        .find(',')
        .ok_or_else(|| "无效Data URL".to_string())?;
    let encoded = &data_url[(comma + 1)..];
    let bytes = b64_engine::STANDARD
        .decode(encoded)
        .map_err(|e| format!("Base64解码失败: {}", e))?;
    let img = image::load_from_memory(&bytes)
        .map_err(|e| format!("解析PNG失败: {}", e))?
        .to_rgba8();
    let (width, height) = img.dimensions();

    // 将RGBA数据转换为BGRA，并进行预乘alpha，保持行顺序（顶部->底部）
    let mut bgra: Vec<u8> = Vec::with_capacity((width * height * 4) as usize);
    for px in img.pixels() {
        let [r, g, b, a] = px.0;
        if a == 0 {
            bgra.extend_from_slice(&[0, 0, 0, 0]);
        } else {
            // 预乘alpha
            let b_p = (b as u16 * a as u16 / 255) as u8;
            let g_p = (g as u16 * a as u16 / 255) as u8;
            let r_p = (r as u16 * a as u16 / 255) as u8;
            bgra.extend_from_slice(&[b_p, g_p, r_p, a]);
        }
    }

    Ok(arboard::ImageData {
        width: width as usize,
        height: height as usize,
        bytes: Cow::Owned(bgra),
    })
}

/// 自动判断文本/图片并设置剪贴板内容
pub fn set_clipboard_content(content: String) -> Result<(), String> {
    set_clipboard_content_internal(content, true)
}

/// 设置剪贴板内容但不添加到历史记录（用于避免重复添加）
pub fn set_clipboard_content_no_history(content: String) -> Result<(), String> {
    set_clipboard_content_internal(content, false)
}

/// 内部函数：设置剪贴板内容
fn set_clipboard_content_internal(content: String, add_to_history: bool) -> Result<(), String> {
    if content.starts_with("data:image/") {
        #[cfg(windows)]
        {
            let (bgra, png_bytes, width, height) = data_url_to_bgra_and_png(&content)?;
            set_windows_clipboard_image(&bgra, &png_bytes, width, height)?;
        }
        #[cfg(not(windows))]
        {
            let image_data = data_url_to_image(&content)?;
            match Clipboard::new() {
                Ok(mut clipboard) => {
                    clipboard
                        .set_image(image_data)
                        .map_err(|e| format!("设置剪贴板图片失败: {}", e))?;
                }
                Err(e) => return Err(format!("获取剪贴板失败: {}", e)),
            }
        }
    } else if content.starts_with("image:") {
        // 处理图片引用格式 "image:id"
        let image_id = content.strip_prefix("image:").unwrap_or("");

        // 从图片管理器获取图片数据
        use crate::image_manager::get_image_manager;
        let image_manager = get_image_manager()?;
        let manager = image_manager
            .lock()
            .map_err(|e| format!("获取图片管理器锁失败: {}", e))?;
        let data_url = manager.get_image_data_url(image_id)?;

        // 递归调用处理data URL
        drop(manager); // 释放锁
        return set_clipboard_content_internal(data_url, add_to_history);
    } else {
        match Clipboard::new() {
            Ok(mut clipboard) => {
                clipboard
                    .set_text(content.clone())
                    .map_err(|e| format!("设置剪贴板文本失败: {}", e))?;
            }
            Err(e) => return Err(format!("获取剪贴板失败: {}", e)),
        }
    }

    // 只有在需要时才添加到历史记录
    if add_to_history {
        clipboard_history::add_to_history(content);
    }

    Ok(())
}
