use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Upload {
  None,
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
  pub scm_root: PathBuf,
  pub build_shell: String,
  pub git_ssh_key: Option<PathBuf>,
  // should match the format accepted by the `--builders` option to nix
  pub builders: Vec<String>,

  pub log_path: PathBuf,
  pub scm_path: PathBuf,

  pub upload: Upload,

  pub database_url: String,
}
