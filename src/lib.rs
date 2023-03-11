#![feature(extern_types)]

use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
pub use sqlx::error::BoxDynError;
use sqlx::{Executor, FromRow, Postgres};
use std::collections::HashMap;
use std::path::PathBuf;

// pub mod logger;
// mod util;
// pub use util::*;
// mod pidfile;
// pub use pidfile::*;

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
          outputs: outputs_group.remove(&i.id).map_or(vec![], |x| x.collect()),
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

#[derive(Debug, Deserialize, Default)]
pub struct WorkerConfig {
  pub log_path: PathBuf,
  pub scm_path: PathBuf,
  pub cache_bucket: String,
  pub s3_region: String,
  pub lockfile: PathBuf,
  pub signing_key: String,
  pub aws_access_key: String,
  pub aws_secret_key: String,
  pub ssh_private_key: String,
  pub shell: String,
  // should match the format accepted by the `--builders` option to nix
  pub builders: Vec<String>,
}

impl WorkerConfig {
  pub fn logfile(&self, id: i32) -> PathBuf {
    self.log_path.join(format!("{}.log", id))
  }
}
