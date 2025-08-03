use image::GenericImageView;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, MenuEvent},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Emitter,
};

pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // 创建托盘菜单
    let toggle_item = MenuItem::with_id(app, "toggle", "显示/隐藏", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let settings_item = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&toggle_item, &separator, &settings_item, &quit_item],
    )?;

    // 创建托盘图标
    let icon = {
        // 优先用 64x64 图标
        let icon_data = include_bytes!("../icons/icon64.png");
        let img = image::load_from_memory(icon_data)?;
        let rgba = img.to_rgba8();
        let (width, height) = img.dimensions();
        tauri::image::Image::new_owned(rgba.into_raw(), width, height)
    };

    let app_handle = app.clone();
    let _tray = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .tooltip("快速剪贴板")
        .icon(icon)
        .menu_on_left_click(false)
        .on_tray_icon_event(move |_tray, event| {
            if let TrayIconEvent::Click { button, .. } = event {
                if button == tauri::tray::MouseButton::Left {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        crate::window_management::show_webview_window(window);
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}
