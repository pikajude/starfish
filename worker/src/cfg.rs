use std::borrow::Cow;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Publish {
  #[serde(alias = "none")]
  None,
  #[serde(alias = "s3")]
  S3 {
    bucket: String,
    region: String,
    access_key: String,
    secret_key: String,
    nix_signing_key: String,
  },
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
  pub build_shell: String,
  pub git_ssh_key: Option<PathBuf>,
  // should match the format accepted by the `--builders` option to nix
  pub builders: Vec<String>,
  #[serde(default = "default_target_platforms")]
  pub target_platforms: Vec<Cow<'static, str>>,

  pub log_path: PathBuf,
  pub scm_path: PathBuf,

  pub publish: Publish,

  pub database_url: String,
}

fn default_target_platforms() -> Vec<Cow<'static, str>> {
  ["x86_64-linux", "x86_64-darwin"]
    .into_iter()
    .map(|x| x.into())
    .collect::<Vec<_>>()
}
