use std::collections::HashMap;
use std::fmt::Display;
use std::path::Path;

use chrono::{DateTime, Utc};
use itertools::Itertools;
use log::{debug, info};
use serde::{Deserialize, Serialize};
pub use sqlx::error::BoxDynError;
use sqlx::{Executor, FromRow, Postgres};

#[derive(Debug, Serialize, FromRow)]
pub struct Build {
  pub id: i32,
  pub origin: String,
  pub rev: String,
  pub created_at: DateTime<Utc>,
  pub status: BuildStatus,
  pub finished_at: Option<DateTime<Utc>>,
  pub error_msg: Option<String>,
}

#[derive(Serialize)]
pub struct InputOutputs {
  #[serde(flatten)]
  input: Input,
  outputs: Vec<Output>,
}

impl Build {
  pub async fn get<'e, 'c: 'e, E>(id: i32, executor: E) -> sqlx::Result<Option<Self>>
  where
    E: 'e + Executor<'c, Database = Postgres>,
  {
    sqlx::query_as!(
      Self,
      "SELECT id, origin, rev, created_at, status as \"status: _\", finished_at, error_msg FROM \
       builds WHERE id = $1",
      id
    )
    .fetch_optional(executor)
    .await
  }

  pub async fn get_inputs_and_outputs<'e, 'c: 'e, E>(
    &self,
    db: E,
  ) -> sqlx::Result<Vec<InputOutputs>>
  where
    E: 'e + Executor<'c, Database = Postgres> + Copy,
  {
    let inputs = sqlx::query_as!(Input, "SELECT * FROM inputs WHERE build_id = $1", self.id)
      .fetch_all(db)
      .await?;

    let outputs = sqlx::query_as!(
      Output,
      "SELECT * FROM outputs WHERE input_id = any ($1)",
      &inputs.iter().map(|x| x.id).collect::<Vec<_>>()
    )
    .fetch_all(db)
    .await?;

    let outputs_group = outputs.into_iter().group_by(|x| x.input_id);
    let mut outputs_group = outputs_group.into_iter().collect::<HashMap<_, _>>();

    Ok(
      inputs
        .into_iter()
        .map(|i| InputOutputs {
          outputs: outputs_group
            .remove(&i.id)
            .map_or(vec![], std::iter::Iterator::collect),
          input: i,
        })
        .collect::<Vec<_>>(),
    )
  }
}

#[derive(Debug, Serialize, FromRow)]
pub struct Input {
  pub id: i32,
  pub build_id: i32,
  pub path: String,
}

#[derive(Debug, Serialize, FromRow, Clone)]
pub struct Output {
  pub id: i32,
  pub input_id: i32,
  pub system: String,
  pub store_path: String,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "build_status", rename_all = "lowercase")]
pub enum BuildStatus {
  // aka "no status"
  Queued,
  Building,
  Uploading,
  Succeeded,
  Failed,
  Canceled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Component {
  Web = 0,
  Worker = 1,
}

impl Display for Component {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Web => write!(f, "web"),
      Self::Worker => write!(f, "worker"),
    }
  }
}

pub static STARFISH_GIT_SHA: &str = env!("VERGEN_GIT_SHA");

static CFG_DEFAULT: [&str; 2] = [
  include_str!("../../config/web.sample.toml"),
  include_str!("../../config/worker.sample.toml"),
];

pub fn load_config<T: serde::de::DeserializeOwned + std::fmt::Debug>(
  component: Component,
) -> anyhow::Result<T> {
  use config::{Config, Environment, File, FileFormat};

  let config_root = std::env::var("STARFISH_CONFIG_DIR").unwrap_or_else(|_| "config/dev".into());
  let config_path = Path::new(&config_root)
    .join(component.to_string())
    .with_extension("toml");

  if !config_path.exists() {
    info!(
      "Configuration file {} does not exist, populating with default values",
      config_path.display()
    );
    std::fs::create_dir_all(config_path.parent().unwrap())?;
    std::fs::write(&config_path, CFG_DEFAULT[component as usize])?;
  }

  let config_contents = std::fs::read_to_string(&config_path)?;

  let cfg_ = Config::builder()
    .add_source(File::from_str(&config_contents, FileFormat::Toml))
    .add_source(
      Environment::with_prefix("starfish")
        .separator(".")
        .prefix_separator("."),
    )
    .build()?;

  Ok(cfg_.try_deserialize().map(|x| {
    debug!("Loaded configuration: {:#?}", x);
    x
  })?)
}

pub fn init_logger() {
  env_logger::init_from_env(env_logger::Env::default().filter_or(
    "STARFISH_LOG",
    if cfg!(debug_assertions) {
      "debug"
    } else {
      "info"
    },
  ));
}
