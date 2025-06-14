use crate::config::model::Config;

pub fn load_config() -> Result<Config, anyhow::Error> {
    load_config_from_path("config.toml")
}

pub fn load_config_from_path(config_path: &str) -> Result<Config, anyhow::Error> {
    let config_str = std::fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_str)?;
    Ok(config)
}
