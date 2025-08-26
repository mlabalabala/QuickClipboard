pub mod screenshot_state;
pub mod screenshot_window;
pub mod screenshot_render;
pub mod screenshot_events;
pub mod screenshot_utils;

// 重新导出主要类型和函数
pub use screenshot_state::{NativeScreenshotState, SelectionRect, ResizeHandle};
pub use screenshot_window::NativeScreenshotWindow;
pub use screenshot_utils::{crop_and_save_to_clipboard, image_to_data_url_with_quality};

// 导出主要命令函数
pub use screenshot_window::start_native_screenshot;
