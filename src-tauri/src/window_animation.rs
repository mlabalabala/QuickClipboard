use std::time::{Duration, Instant};
use tauri::{async_runtime::spawn, PhysicalPosition, PhysicalSize, Position, WebviewWindow};

#[tauri::command]
pub fn animate_window_resize(
    window: WebviewWindow,
    start_width: f64,
    start_height: f64,
    start_x: i32,
    start_y: i32,
    end_width: f64,
    end_height: f64,
    end_x: i32,
    end_y: i32,
    duration_ms: u64,
) -> Result<(), String> {
    spawn(async move {
        let start_time = Instant::now();
        let duration = Duration::from_millis(duration_ms);
        let frame_time = Duration::from_millis(16);

        let d_width = end_width - start_width;
        let d_height = end_height - start_height;
        let d_x = end_x - start_x;
        let d_y = end_y - start_y;

        loop {
            let elapsed = start_time.elapsed();
            let progress = (elapsed.as_secs_f64() / duration.as_secs_f64()).min(1.0);

            let eased = if progress < 0.5 {
                4.0 * progress * progress * progress
            } else {
                1.0 - (-2.0 * progress + 2.0).powi(3) / 2.0
            };

            let current_width = start_width + d_width * eased;
            let current_height = start_height + d_height * eased;
            let current_x = start_x + (d_x as f64 * eased) as i32;
            let current_y = start_y + (d_y as f64 * eased) as i32;

            let _ = window.set_size(PhysicalSize::new(current_width as u32, current_height as u32));
            let _ = window.set_position(Position::Physical(PhysicalPosition::new(current_x, current_y)));

            if progress >= 1.0 {
                break;
            }

            tokio::time::sleep(frame_time).await;
        }
    });

    Ok(())
}
