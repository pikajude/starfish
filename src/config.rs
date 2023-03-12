use ::config::{Config, Environment, File, FileFormat};

use crate::WorkerConfig;

pub fn load() -> Result<WorkerConfig, ::config::ConfigError> {
  let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

  let cfg_ = Config::builder()
    .add_source(File::new("config/default", FileFormat::Toml))
    .add_source(File::new(&format!("config/{run_mode}"), FileFormat::Toml))
    .add_source(Environment::with_prefix("starfish"))
    .build()?;

  cfg_.try_deserialize()
}
