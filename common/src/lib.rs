use std::collections::HashMap;

use anyhow::Context;
use chrono::{DateTime, Utc};
use itertools::Itertools;
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

pub static STARFISH_VERSION: &str = env!("VERGEN_GIT_SHA");

pub fn load_config<T: serde::de::DeserializeOwned + std::fmt::Debug>(
  cfg_path: &str,
) -> anyhow::Result<T> {
  use config::{Config, Environment, File, FileFormat};

  let run_mode = std::env::var("STARFISH_RUN_MODE").unwrap_or_else(|_| "development".into());

  let cfg_ = Config::builder()
    .add_source(File::new(cfg_path, FileFormat::Toml).required(false))
    .add_source(File::new(&format!("{cfg_path}.{run_mode}"), FileFormat::Toml).required(false))
    .add_source(Environment::with_prefix("starfish"))
    .build()?;

  cfg_
    .try_deserialize()
    .map(|x| {
      log::debug!("Loaded configuration: {:#?}", x);
      x
    })
    .with_context(|| format!("Error loading configuration '{cfg_path}'"))
}
