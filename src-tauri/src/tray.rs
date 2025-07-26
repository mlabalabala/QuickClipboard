use image::GenericImageView;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle,
};

pub fn setup_tray(app_handle: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    println!("设置托盘图标...");

    // 创建托盘菜单
    let toggle_item = MenuItem::with_id(app_handle, "toggle", "显示/隐藏", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app_handle)?;
    let settings_item = MenuItem::with_id(app_handle, "settings", "设置", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app_handle, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(
        app_handle,
        &[&toggle_item, &separator, &settings_item, &quit_item],
    )?;

    // 创建托盘图标
    let icon = load_app_icon()?;
    let _tray = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .tooltip("快速剪贴板")
        .icon(icon)
        .build(app_handle)?;

    println!("托盘图标创建成功");
    Ok(())
}

fn load_app_icon() -> Result<Image<'static>, Box<dyn std::error::Error>> {
    // 首先尝试加载 32x32 图标
    if let Some(image) = try_load_embedded_icon() {
        println!("成功加载内嵌应用图标");
        return Ok(image);
    }

    // 如果无法加载图标文件，回退到手绘图标
    println!("无法加载应用图标文件，使用默认图标");
    create_fallback_icon()
}

fn try_load_embedded_icon() -> Option<Image<'static>> {
    // 尝试加载内嵌的图标
    if let Ok(icon_bytes) = std::fs::read("icons/icon64.png") {
        // 使用 image crate 来解析 PNG 文件
        if let Ok(img) = image::load_from_memory(&icon_bytes) {
            let rgba = img.to_rgba8();
            let (width, height) = img.dimensions();
            let rgba_data = rgba.into_raw();

            let tauri_image = Image::new_owned(rgba_data, width, height);
            return Some(tauri_image);
        }
    }
    None
}

fn create_fallback_icon() -> Result<Image<'static>, Box<dyn std::error::Error>> {
    // 创建一个16x16像素的剪贴板图标作为备用
    let size = 16;
    let mut rgba = vec![0u8; size * size * 4];

    // 定义颜色
    let bg_color = [0, 0, 0, 0]; // 透明背景
    let clipboard_color = [70, 130, 220, 255]; // 蓝色剪贴板
    let paper_color = [255, 255, 255, 255]; // 白色纸张
    let text_color = [100, 100, 100, 255]; // 灰色文本线条

    // 绘制剪贴板图标
    for y in 0..size {
        for x in 0..size {
            let index = (y * size + x) * 4;
            let mut color = bg_color;

            // 绘制剪贴板外框 (3-13, 2-14)
            if x >= 3 && x <= 13 && y >= 2 && y <= 14 {
                // 剪贴板夹子 (6-10, 1-3)
                if x >= 6 && x <= 10 && y >= 1 && y <= 3 {
                    color = clipboard_color;
                }
                // 剪贴板边框
                else if x == 3 || x == 13 || y == 2 || y == 14 {
                    color = clipboard_color;
                }
                // 纸张内容区域 (4-12, 3-13)
                else if x >= 4 && x <= 12 && y >= 3 && y <= 13 {
                    color = paper_color;

                    // 绘制文本线条
                    if (y == 5 || y == 7 || y == 9 || y == 11) && x >= 5 && x <= 11 {
                        color = text_color;
                    }
                }
            }

            rgba[index] = color[0]; // R
            rgba[index + 1] = color[1]; // G
            rgba[index + 2] = color[2]; // B
            rgba[index + 3] = color[3]; // A
        }
    }

    let image = Image::new_owned(rgba, size as u32, size as u32);
    Ok(image)
}
