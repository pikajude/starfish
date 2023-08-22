use std::fmt::Display;
use std::fs::File;
use std::io::Write;
use std::process::{Command, ExitStatus, Output};

/// This struct exists so you can execute commands that pipe to a file while
/// also returning the output to the calling process (us). It only really works
/// for processes that do all their debug logging to stderr. Fortunately
/// nix-build does that.
pub struct Logger {
  fd: File,
}

impl From<File> for Logger {
  fn from(fd: File) -> Self {
    Self { fd }
  }
}

impl Logger {
  pub fn exec(&mut self, cmd: &mut Command) -> std::io::Result<ExitStatus> {
    self.debug(cmd)?;
    cmd
      .env("PATH", std::env::var_os("PATH").expect("PATH not set"))
      .stderr(self.fd.try_clone()?)
      .stdout(self.fd.try_clone()?)
      .status()
  }

  pub fn output(&mut self, cmd: &mut Command) -> std::io::Result<Output> {
    self.debug(cmd)?;
    let out = cmd
      .env("PATH", std::env::var_os("PATH").expect("PATH not set"))
      .stderr(self.fd.try_clone()?)
      .output()?;

    self.fd.write_all(&out.stdout)?;

    Ok(out)
  }

  pub fn log<D: Display>(&mut self, message: D) -> std::io::Result<()> {
    writeln!(self.fd, "{message}")
  }

  pub fn fake_exec<D: Display>(&mut self, cmd: D) -> std::io::Result<()> {
    self.log(format!("$ {cmd}"))
  }

  fn debug(&mut self, cmd: &Command) -> std::io::Result<()> {
    write!(self.fd, "$ {}", cmd.get_program().to_string_lossy())?;
    for arg in cmd.get_args() {
      write!(self.fd, " {}", arg.to_string_lossy())?;
    }
    writeln!(self.fd)
  }
}
