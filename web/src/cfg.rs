use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::str::FromStr;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
  pub log_path: PathBuf,
  pub static_root: PathBuf,
  pub database_url: String,
  pub listen_address: String,
  pub listen_port: u16,
}

impl Config {
  pub fn logfile(&self, id: i32) -> PathBuf {
    self.log_path.join(format!("{id}.log"))
  }

  pub fn listen_addr(&self) -> Result<SocketAddr, <IpAddr as FromStr>::Err> {
    Ok(SocketAddr::from((
      self.listen_address.parse::<IpAddr>()?,
      self.listen_port,
    )))
  }
}
