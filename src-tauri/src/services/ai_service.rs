/// AI服务 - 处理AI相关的业务逻辑
pub struct AIService;

impl AIService {
    /// 获取可用的AI模型列表
    pub async fn get_available_ai_models() -> Result<Vec<String>, String> {
        let settings = crate::settings::get_global_settings();
        let ai_config = crate::ai_config::create_ai_config_from_settings(&settings);

        if !ai_config.is_valid() {
            return Err("AI配置无效，请检查API密钥等设置".to_string());
        }

        // 简化实现，返回常用模型列表
        Ok(vec![
            "gpt-3.5-turbo".to_string(),
            "gpt-4".to_string(),
            "gpt-4-turbo".to_string(),
        ])
    }
}
