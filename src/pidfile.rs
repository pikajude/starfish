use std::path::Path;

use log::error;
use nix::errno::Errno;
use nix::NixPath;

#[link(name = "bsd", kind = "static")]
extern "C" {
  type pidfh;

  fn pidfile_open(
    path: *const libc::c_char,
    mode: libc::mode_t,
    pidptr: *const libc::pid_t,
  ) -> *mut pidfh;

  fn pidfile_write(pfh: *mut pidfh) -> libc::c_int;
  fn pidfile_remove(pfh: *mut pidfh) -> libc::c_int;
}

pub struct Pidfile(*mut pidfh);

impl Pidfile {
  pub fn new<P: AsRef<Path>>(path: P) -> Self {
    let path = path.as_ref();
    match Self::try_new(path) {
      Ok(x) => x,
      Err(e) => {
        error!(
          "unable to lock file {} ({}). is another process using it?",
          path.display(),
          e
        );
        std::process::exit(1)
      }
    }
  }

  fn try_new(path: &Path) -> nix::Result<Self> {
    let pf =
      path.with_nix_path(|p| unsafe { pidfile_open(p.as_ptr(), 0o600, std::ptr::null()) })?;
    if pf.is_null() {
      return Err(Errno::last());
    }

    if unsafe { pidfile_write(pf) } != 0 {
      return Err(Errno::last());
    }

    Ok(Self(pf))
  }
}

impl Drop for Pidfile {
  fn drop(&mut self) {
    if self.0.is_null() {
      return;
    }
    unsafe {
      pidfile_remove(self.0);
    }
  }
}
