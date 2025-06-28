use crate::config::model::Config;
use std::path::Path;

/// 加载配置文件，支持多种配置路径来源
///
/// 优先级顺序：
/// 1. 环境变量 CONFIG_PATH
/// 2. 命令行参数（如果有的话）
/// 3. 默认路径 config.toml
/// 4. 备用路径 config-example.toml
pub fn load_config() -> Result<Config, anyhow::Error> {
    // 1. 首先检查环境变量
    if let Ok(config_path) = std::env::var("CONFIG_PATH") {
        tracing::info!(
            "Loading config from environment variable CONFIG_PATH: {}",
            config_path
        );
        return load_config_from_path(&config_path);
    }

    // 2. 检查默认配置文件
    let default_paths = ["config.toml"];

    for path in &default_paths {
        if Path::new(path).exists() {
            tracing::info!("Loading config from default path: {}", path);
            return load_config_from_path(path);
        }
    }

    // 3. 如果都没找到，返回错误并提供帮助信息
    Err(anyhow::anyhow!(
        "Configuration file not found. Please:\n\
         1. Set CONFIG_PATH environment variable, or\n\
         2. Create config.toml in current directory, or\n\
         3. Copy from template: cp config-example.toml config.toml"
    ))
}

/// 从指定路径加载配置文件
pub fn load_config_from_path(config_path: &str) -> Result<Config, anyhow::Error> {
    // 检查文件是否存在
    if !Path::new(config_path).exists() {
        return Err(anyhow::anyhow!(
            "Configuration file not found: {}\n\
             Please check the file path or create the configuration file.",
            config_path
        ));
    }

    // 读取并解析配置文件
    let config_str = std::fs::read_to_string(config_path)
        .map_err(|e| anyhow::anyhow!("Failed to read config file '{}': {}", config_path, e))?;

    let config: Config = toml::from_str(&config_str)
        .map_err(|e| anyhow::anyhow!("Failed to parse config file '{}': {}", config_path, e))?;

    tracing::info!("Configuration loaded successfully from: {}", config_path);
    Ok(config)
}

/// 获取配置文件路径（用于调试和日志）
pub fn get_config_path() -> String {
    if let Ok(config_path) = std::env::var("CONFIG_PATH") {
        config_path
    } else {
        let default_paths = ["config.toml"];

        for path in &default_paths {
            if Path::new(path).exists() {
                return path.to_string();
            }
        }

        "config.toml (not found)".to_string()
    }
}
