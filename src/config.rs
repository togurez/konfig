use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub app_port: u16,
    pub app_env: String,
    pub database_url: String,
    pub database_max_connections: u32,
    /// Shared secret validated on every inbound request via X-Internal-Token.
    pub internal_api_secret: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();
        let cfg = config::Config::builder()
            .set_default("app_port", 8080)?
            .set_default("app_env", "development")?
            .set_default("database_max_connections", 10)?
            .add_source(config::Environment::default())
            .build()?
            .try_deserialize()?;
        Ok(cfg)
    }
}
