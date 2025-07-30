// 服务层模块
// 
// 服务层负责整合各个功能模块，提供高级的业务逻辑接口
// 避免在 commands.rs 中直接实现复杂的业务逻辑

pub mod translation_service;
pub mod paste_service;
pub mod window_service;
