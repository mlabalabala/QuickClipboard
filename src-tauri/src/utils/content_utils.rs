// 内容处理工具函数

// 生成文件类型的标题
pub fn generate_files_title(files_content: &str) -> String {
    // 解析文件数据
    if let Some(files_json) = files_content.strip_prefix("files:") {
        if let Ok(files_data) = serde_json::from_str::<serde_json::Value>(files_json) {
            if let Some(files_array) = files_data["files"].as_array() {
                let file_count = files_array.len();

                if file_count == 0 {
                    return "空文件列表".to_string();
                } else if file_count == 1 {
                    // 单个文件，显示文件名
                    if let Some(file_name) = files_array[0]["name"].as_str() {
                        return file_name.to_string();
                    }
                } else {
                    // 多个文件，显示第一个文件名和数量
                    if let Some(first_file_name) = files_array[0]["name"].as_str() {
                        return format!("{} 等 {} 个文件", first_file_name, file_count);
                    } else {
                        return format!("{} 个文件", file_count);
                    }
                }
            }
        }
    }

    // 解析失败时的回退标题
    "文件".to_string()
}
