/// 文件对话框服务 - 处理文件选择对话框相关的业务逻辑
pub struct FileDialogService;

impl FileDialogService {
    /// 浏览音效文件
    pub async fn browse_sound_file() -> Result<Option<String>, String> {
        let dialog = rfd::AsyncFileDialog::new()
            .add_filter("音频文件", &["wav", "mp3", "ogg", "flac", "m4a", "aac"])
            .set_title("选择音效文件");

        if let Some(file) = dialog.pick_file().await {
            Ok(Some(file.path().to_string_lossy().to_string()))
        } else {
            Ok(None)
        }
    }

    /// 浏览背景图片文件
    pub async fn browse_image_file() -> Result<Option<String>, String> {
        let dialog = rfd::AsyncFileDialog::new()
            .add_filter("图片文件", &["png", "jpg", "jpeg", "bmp", "gif", "webp"])
            .set_title("选择背景图片");

        if let Some(file) = dialog.pick_file().await {
            Ok(Some(file.path().to_string_lossy().to_string()))
        } else {
            Ok(None)
        }
    }
}
