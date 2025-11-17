use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScannerError {
    #[error("API 请求失败: {0}")]
    ApiError(#[from] reqwest::Error),
    
    #[error("JSON 解析错误: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("无效的响应数据: {0}")]
    InvalidResponse(String),
    
    #[error("网络错误: {0}")]
    #[allow(dead_code)]
    NetworkError(String),
    
    #[error("配置错误: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, ScannerError>;

