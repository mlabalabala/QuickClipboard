use crate::clipboard_content::set_clipboard_content_no_history;
use base64::{engine::general_purpose, Engine as _};
use image::{DynamicImage, ImageFormat};
use screenshots::Screen;

use std::io::Cursor;
use tauri::Manager;

/// 截屏区域参数
#[derive(serde::Deserialize)]
pub struct ScreenshotArea {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// 创建截屏窗口
#[tauri::command]
pub async fn open_screenshot_window(app: tauri::AppHandle) -> Result<(), String> {
    // 先隐藏主窗口
    if let Some(main_window) = app.get_webview_window("main") {
        main_window
            .hide()
            .map_err(|e| format!("隐藏主窗口失败: {}", e))?;
    }

    // 如果截屏窗口已存在，先彻底关闭它
    if let Some(screenshot_window) = app.get_webview_window("screenshot") {
        println!("发现已存在的截屏窗口，正在关闭...");
        if let Err(e) = screenshot_window.close() {
            println!("关闭已存在的截屏窗口失败: {}", e);
        } else {
            println!("已存在的截屏窗口关闭成功");
        }

        // 等待窗口完全关闭
        std::thread::sleep(std::time::Duration::from_millis(100));

        // 再次检查窗口是否真的被关闭了
        if let Some(_) = app.get_webview_window("screenshot") {
            println!("警告: 截屏窗口仍然存在，尝试强制销毁");
            // 如果窗口仍然存在，需要等待更长时间或使用不同的标签
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    } else {
        println!("没有发现已存在的截屏窗口");
    }

    // 重新创建截屏窗口以确保正确的多屏幕支持
    {
        // 获取所有屏幕信息，计算总的桌面区域
        let screens = Screen::all().map_err(|e| format!("获取屏幕信息失败: {}", e))?;

        // 计算所有屏幕的边界
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;

        for screen in &screens {
            let display = &screen.display_info;
            min_x = min_x.min(display.x);
            min_y = min_y.min(display.y);
            max_x = max_x.max(display.x + display.width as i32);
            max_y = max_y.max(display.y + display.height as i32);
        }

        let total_width = (max_x - min_x) as f64;
        let total_height = (max_y - min_y) as f64;

        println!(
            "创建截屏窗口: 位置=({}, {}), 尺寸=({}, {})",
            min_x, min_y, total_width, total_height
        );

        // 创建覆盖所有屏幕的截屏窗口
        println!("开始创建截屏窗口...");

        // 使用时间戳确保窗口标签唯一
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let window_label = format!("screenshot_{}", timestamp);
        println!("使用窗口标签: {}", window_label);

        let screenshot_window = match tauri::WebviewWindowBuilder::new(
            &app,
            &window_label,
            tauri::WebviewUrl::App("screenshot.html".into()),
        )
        .title("截屏选择")
        .inner_size(total_width, total_height)
        .position(min_x as f64, min_y as f64)
        .fullscreen(false) // 不使用全屏模式，而是手动设置大小和位置
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .shadow(false) // 禁用窗口阴影
        .build()
        {
            Ok(window) => {
                println!("截屏窗口创建成功");
                window
            }
            Err(e) => {
                println!("截屏窗口创建失败: {}", e);
                return Err(format!("创建截屏窗口失败: {}", e));
            }
        };

        println!(
            "截屏窗口构建完成: 位置=({}, {}), 尺寸=({}, {})",
            min_x, min_y, total_width, total_height
        );

        // 首次显示窗口
        if let Err(e) = screenshot_window.show() {
            println!("首次显示窗口失败: {}", e);
            return Err(format!("显示截屏窗口失败: {}", e));
        }

        // 确保窗口完全显示
        if let Err(e) = screenshot_window.set_always_on_top(true) {
            println!("设置窗口置顶失败: {}", e);
            return Err(format!("设置窗口置顶失败: {}", e));
        }

        // 等待窗口创建完成后，再次精确设置位置和显示状态
        std::thread::sleep(std::time::Duration::from_millis(200));

        // 使用 Tauri API 直接设置窗口位置，更可靠
        if let Err(e) =
            screenshot_window.set_position(tauri::Position::Physical(tauri::PhysicalPosition {
                x: min_x,
                y: min_y,
            }))
        {
            println!("设置窗口位置失败: {}", e);
            return Err(format!("设置窗口位置失败: {}", e));
        }

        // 再次确保窗口显示
        if let Err(e) = screenshot_window.show() {
            println!("再次显示窗口失败: {}", e);
            return Err(format!("再次显示截屏窗口失败: {}", e));
        }

        // 设置为工具窗口（不获取焦点且置顶）
        #[cfg(windows)]
        {
            if let Err(e) = crate::window_management::set_super_topmost_window(&screenshot_window) {
                println!("设置截屏窗口为工具窗口失败: {}", e);
                return Err(format!("设置截屏窗口为工具窗口失败: {}", e));
            }
        }

        println!(
            "截屏窗口已创建并显示: 位置=({}, {}), 尺寸=({}, {})",
            min_x, min_y, total_width, total_height
        );
    }

    Ok(())
}

/// 关闭截屏窗口并显示主窗口
#[tauri::command]
pub async fn close_screenshot_window(app: tauri::AppHandle) -> Result<(), String> {
    // 关闭所有截屏相关的窗口
    let webview_windows = app.webview_windows();
    let mut closed_any = false;

    for (label, window) in webview_windows {
        if label.starts_with("screenshot") {
            println!("关闭截屏窗口: {}", label);
            if let Err(e) = window.close() {
                println!("关闭截屏窗口 {} 失败: {}", label, e);
            } else {
                println!("截屏窗口 {} 关闭成功", label);
                closed_any = true;
            }
        }
    }

    if !closed_any {
        println!("没有找到需要关闭的截屏窗口");
    }

    // 显示主窗口并正确设置鼠标监听
    if let Some(main_window) = app.get_webview_window("main") {
        // 智能定位窗口到光标位置
        #[cfg(windows)]
        {
            let _ = crate::window_management::position_window_at_cursor(&main_window);
        }

        main_window
            .show()
            .map_err(|e| format!("显示主窗口失败: {}", e))?;

        #[cfg(windows)]
        {
            let _ = crate::window_management::set_super_topmost_window(&main_window);
            // 启用鼠标监听，这样点击外部可以隐藏窗口
            crate::mouse_hook::enable_mouse_monitoring();
        }
    }

    Ok(())
}

/// 区域截屏
#[tauri::command]
pub async fn take_screenshot(
    app: tauri::AppHandle,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(), String> {
    // 直接使用原始坐标，不进行DPI调整
    println!(
        "接收到的截屏坐标: x={}, y={}, width={}, height={}",
        x, y, width, height
    );

    // 找到截屏窗口并临时隐藏，但要最小化屏幕闪烁
    let screenshot_window = app
        .webview_windows()
        .into_iter()
        .find(|(label, _)| label.starts_with("screenshot"))
        .map(|(_, window)| window);

    if let Some(window) = &screenshot_window {
        // 快速隐藏窗口，减少闪烁
        let _ = window.hide();
    }

    // 很短的等待时间，确保窗口隐藏但减少闪烁
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let result = capture_screen_area(x, y, width, height).await;

    // 无论截屏成功还是失败，都要关闭截屏窗口并显示主窗口
    if let Err(e) = close_screenshot_window(app).await {
        println!("关闭截屏窗口失败: {}", e);
    }

    result
}

/// 全屏截屏
#[tauri::command]
pub async fn take_fullscreen_screenshot(app: tauri::AppHandle) -> Result<(), String> {
    // 先关闭截屏窗口以避免截到窗口本身
    if let Some(screenshot_window) = app.get_webview_window("screenshot") {
        screenshot_window
            .hide()
            .map_err(|e| format!("隐藏截屏窗口失败: {}", e))?;
    }

    // 等待一小段时间确保窗口完全隐藏
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let result = capture_fullscreen().await;

    // 关闭截屏窗口并显示主窗口
    let _ = close_screenshot_window(app).await;

    result
}

/// 捕获指定区域的屏幕截图
async fn capture_screen_area(x: i32, y: i32, width: u32, height: u32) -> Result<(), String> {
    let screens = Screen::all().map_err(|e| format!("获取屏幕信息失败: {}", e))?;

    // 找到包含指定区域的屏幕
    let mut target_screen = None;
    for screen in &screens {
        let display = &screen.display_info;
        let screen_right = display.x + display.width as i32;
        let screen_bottom = display.y + display.height as i32;

        // 检查截屏区域是否与此屏幕重叠
        if x < screen_right
            && (x + width as i32) > display.x
            && y < screen_bottom
            && (y + height as i32) > display.y
        {
            target_screen = Some(screen);
            break;
        }
    }

    let screen = target_screen.ok_or("未找到包含指定区域的屏幕")?;

    // 将全局坐标转换为屏幕相对坐标
    let relative_x = x - screen.display_info.x;
    let relative_y = y - screen.display_info.y;

    println!(
        "屏幕信息: x={}, y={}, width={}, height={}",
        screen.display_info.x,
        screen.display_info.y,
        screen.display_info.width,
        screen.display_info.height
    );
    println!(
        "相对坐标: x={}, y={}, width={}, height={}",
        relative_x, relative_y, width, height
    );

    // 检查坐标是否有效
    if relative_x < 0 || relative_y < 0 {
        return Err(format!(
            "无效的相对坐标: x={}, y={}",
            relative_x, relative_y
        ));
    }

    // 截取指定区域
    let image = screen
        .capture_area(relative_x, relative_y, width, height)
        .map_err(|e| format!("截屏失败: {}", e))?;

    // 转换为PNG格式的base64数据URL
    let dynamic_image = DynamicImage::ImageRgba8(image);
    let data_url = image_to_data_url(&dynamic_image)?;

    // 保存到剪贴板（不添加到历史记录，让监听器处理）
    set_clipboard_content_no_history(data_url)
        .map_err(|e| format!("保存截屏到剪贴板失败: {}", e))?;

    Ok(())
}

/// 捕获全屏截图
async fn capture_fullscreen() -> Result<(), String> {
    let screens = Screen::all().map_err(|e| format!("获取屏幕信息失败: {}", e))?;
    let primary_screen = screens.first().ok_or("未找到主屏幕")?;

    // 截取全屏
    let image = primary_screen
        .capture()
        .map_err(|e| format!("全屏截屏失败: {}", e))?;

    // 转换为PNG格式的base64数据URL
    let dynamic_image = DynamicImage::ImageRgba8(image);
    let data_url = image_to_data_url(&dynamic_image)?;

    // 保存到剪贴板（不添加到历史记录，让监听器处理）
    set_clipboard_content_no_history(data_url)
        .map_err(|e| format!("保存截屏到剪贴板失败: {}", e))?;

    Ok(())
}

/// 调整坐标以处理DPI缩放
fn adjust_coordinates_for_dpi(
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<(i32, i32, u32, u32), String> {
    // 暂时禁用DPI缩放调整，直接返回原始坐标
    // 因为screenshots库通常已经处理了DPI缩放问题
    println!(
        "截屏坐标: x={}, y={}, width={}, height={}",
        x, y, width, height
    );

    // 如果仍有偏移问题，可能需要在前端进行坐标调整
    Ok((x, y, width, height))
}

/// 将图片转换为data URL格式
fn image_to_data_url(image: &DynamicImage) -> Result<String, String> {
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);

    image
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| format!("图片编码失败: {}", e))?;

    let base64_string = general_purpose::STANDARD.encode(&buffer);
    Ok(format!("data:image/png;base64,{}", base64_string))
}
