#![feature(try_blocks)]

mod cfg;
mod logger;
mod scripts;

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Display;
use std::process::Command;
use std::sync::Arc;

use anyhow::{Context, Result};
use askama::Template;
use cfg::{Config, Publish};
use chrono::Utc;
use common::{Build, BuildStatus, Input};
use futures_util::StreamExt;
use log::info;
use logger::Logger;
use nix::sys::statvfs::{statvfs, Statvfs};
use sha1::{Digest, Sha1};
use sqlx::postgres::PgListener;
use sqlx::PgPool;
use tempfile::TempDir;
use tokio::task::JoinHandle;

struct Worker<'a> {
  cfg: Arc<Config>,
  jobs: HashMap<i32, JoinHandle<()>>,
  fsdata: Statvfs,
  db: &'a PgPool,
}

impl<'a> Worker<'a> {
  fn new(cfg: Config, db: &'a PgPool) -> Result<Self> {
    let fsdata = statvfs("/nix/store")?;

    Ok(Self {
      cfg: Arc::new(cfg),
      jobs: HashMap::new(),
      fsdata,
      db,
    })
  }

  async fn run(mut self) -> Result<()> {
    info!("checking for old unfinished builds");
    // TODO: this only starts builds that never got picked up by the worker. it
    // should restart builds that started running but got reaped or OOM'd or
    // whatever. but that logic is a lot harder.
    let mut unbuilt_builds = sqlx::query!(
      r#"SELECT id FROM builds WHERE status IN ($1,$2,$3)"#,
      BuildStatus::Queued as _,
      BuildStatus::Building as _,
      BuildStatus::Uploading as _
    )
    .fetch(self.db);

    // TODO: rewrite this with try_for_each() or something (the types are annoying)
    while let Some(x) = unbuilt_builds.next().await.transpose()? {
      self.handle("build_restarted", x.id).await?;
    }

    info!("waiting for build notifications");
    let mut listener = PgListener::connect_with(self.db).await?;
    listener
      .listen_all(["build_queued", "build_restarted", "build_canceled"])
      .await?;
    loop {
      let notif = listener.recv().await?;
      let build_id = notif.payload().parse::<i32>()?;
      self.handle(notif.channel(), build_id).await?;
    }
  }

  async fn handle(&mut self, channel: &str, build_id: i32) -> Result<()> {
    info!("got {}: '{}'", channel, build_id);
    if channel == "build_restarted" {
      // delete evidence of old builds so they don't clog up the UI
      sqlx::query!(
        "DELETE FROM outputs WHERE id in (select outputs.id from outputs inner join inputs on \
         outputs.input_id = inputs.id where inputs.build_id = $1)",
        build_id
      )
      .execute(self.db)
      .await?;

      // TODO: we really should keep old logs
      let log_filepath = self.cfg.log_path.join(format!("{build_id}.log"));
      if log_filepath.exists() {
        let _ = File::create(log_filepath)?;
      }

      if let Some(jh) = self.jobs.remove(&build_id) {
        jh.abort();
      }
    }
    self.build(build_id).await
  }

  async fn build(&mut self, build_id: i32) -> Result<()> {
    let build_info = sqlx::query_as!(
      Build,
      "SELECT id, origin, rev, created_at, finished_at, error_msg, status as \"status: _\" FROM \
       builds WHERE id = $1",
      build_id
    )
    .fetch_optional(self.db)
    .await?;
    let Some(r) = build_info else {
      info!("build {} has gone missing, doing nothing", build_id);
      return Ok(());
    };
    self.build_impl(r).await
  }

  async fn build_impl(&mut self, mut build_info: Build) -> Result<()> {
    let log_filepath = self.cfg.log_path.join(format!("{}.log", build_info.id));
    let bid = build_info.id;

    let all_inputs = sqlx::query_as!(Input, "SELECT * FROM inputs WHERE build_id = $1", bid)
      .fetch_all(self.db)
      .await?;

    std::fs::create_dir_all(log_filepath.parent().unwrap())?;
    let mut logger = Logger::from(File::create(&log_filepath)?);

    let mut s = Sha1::new();
    s.update(&build_info.origin);

    macro_rules! status {
      ($stat:expr, $executor:expr) => {
        sqlx::query!(
          "UPDATE builds SET status = $2 WHERE id = $1",
          bid,
          $stat as _
        )
        .execute($executor)
        .await?
      };
    }

    status!(BuildStatus::Building, self.db);

    // create a bare repository in $scm_path, then add a worktree pointing to the
    // right commit. this way we can run builds for multiple commits at the same
    // time. everything from checking path existence to creating the new worktree is
    // "synchronous" (in this process, not across processes, no filesystem locking
    // or anything) and then nix-store and nix-instantiate are called in a separate
    // thread since they make up the bulk of the runtime.
    let scm_dir = self
      .cfg
      .scm_path
      .join(base16ct::lower::encode_string(&s.finalize()));
    if !scm_dir.exists() {
      std::fs::create_dir_all(&scm_dir)?;
      logger.exec(
        Command::new("git")
          .args(["init", "--bare"])
          .current_dir(&scm_dir),
      )?;
      if !logger
        .exec(
          Command::new("git")
            .args(["remote", "add", "origin", "--"])
            .arg(&build_info.origin)
            .current_dir(&scm_dir),
        )?
        .success()
      {
        status!(BuildStatus::Failed, self.db);
        return Ok(());
      }
    }

    let git_ssh_cmd = self.cfg.git_ssh_key.as_ref().map_or_else(
      || "ssh".to_string(),
      |k| {
        format!(
          "ssh -i {} -o IdentitiesOnly=yes -o StrictHostKeyChecking=no",
          k.display()
        )
      },
    );

    if !logger
      .exec(
        Command::new("git")
          .args(["fetch", "origin"])
          .env("GIT_SSH_COMMAND", &git_ssh_cmd)
          .current_dir(&scm_dir),
      )?
      .success()
    {
      status!(BuildStatus::Failed, self.db);
      return Ok(());
    }

    let real_hash = if is_commit_hash(&build_info.rev) {
      if !logger
        .exec(
          Command::new("git")
            .args(["fetch", "origin", &build_info.rev])
            .env("GIT_SSH_COMMAND", &git_ssh_cmd)
            .current_dir(&scm_dir),
        )?
        .success()
      {
        status!(BuildStatus::Failed, self.db);
        return Ok(());
      }
      build_info.rev.clone()
    } else {
      let hash = logger.output(
        Command::new("git")
          .arg("rev-parse")
          .arg(format!("remotes/origin/{}", &build_info.rev))
          .current_dir(&scm_dir),
      )?;
      if !hash.status.success() {
        status!(BuildStatus::Failed, self.db);
        return Ok(());
      }

      String::from_utf8_lossy(&hash.stdout).trim_end().to_string()
    };

    if real_hash != build_info.rev {
      // TODO: this is a dumb way to solve this problem
      sqlx::query!(
        "UPDATE builds SET rev = $2 WHERE id = $1",
        build_info.id,
        &real_hash
      )
      .execute(self.db)
      .await?;
      build_info.rev = real_hash;
    }

    let build_tag = format!("__starfish_build_{}", build_info.id);
    logger.exec(
      Command::new("git")
        .args(["worktree", "remove", "--force"])
        .arg(&build_tag)
        .current_dir(&scm_dir),
    )?;
    if !logger
      .exec(
        Command::new("git")
          .args(["worktree", "add"])
          .arg(&build_tag)
          .arg(&build_info.rev)
          .current_dir(&scm_dir),
      )?
      .success()
    {
      status!(BuildStatus::Failed, self.db);
      return Ok(());
    }

    if !logger
      .exec(Command::new("git").arg("prune").current_dir(&scm_dir))?
      .success()
    {
      status!(BuildStatus::Failed, self.db);
      return Ok(());
    }

    // clone the pool instance so it satisfies 'static
    let finalizer_conn = self.db.clone();
    let cfg = Arc::clone(&self.cfg);

    let filesystem_bytes = self.fsdata.blocks() * self.fsdata.fragment_size();
    // auto GC once the disk is 85% full
    let min_free = filesystem_bytes * 15 / 100;
    // stop GC once the disk is half empty
    let max_free = filesystem_bytes / 2;

    let jh = tokio::spawn(async move {
      let build_err: Result<()> = try {
        // the global config in NIX_CONF_DIR is totally ignored if a user-level config
        // exists. in the docker image, that's not a problem, but while testing locally
        // it's a huge pain
        let nix_superconf_dir = TempDir::new()?;

        let post_build_path = nix_superconf_dir.path().join("post-build.sh");
        let mut post_build = File::create(&post_build_path)?;
        Self::setup_secrets(&cfg.publish, &mut post_build)?;
        // the file must be closed before nix tries to execute it, otherwise weird stuff
        // happens
        drop(post_build);

        std::fs::create_dir(nix_superconf_dir.path().join("nix"))?;
        let mut nix_conf = File::create(nix_superconf_dir.path().join("nix").join("nix.conf"))?;

        let nix_template = NixConf {
          min_free_bytes: min_free,
          max_free_bytes: max_free,
          builders: cfg.builders.join("; "),
          post_build_hook: post_build_path.display(),
        };

        writeln!(nix_conf, "{}", nix_template.render().unwrap())?;

        logger
          .exec(Command::new("cat").arg(nix_superconf_dir.path().join("nix").join("nix.conf")))?;

        logger.fake_exec(format!(
          "export HOME={}",
          nix_superconf_dir.path().display()
        ))?;

        logger.exec(Command::new("chmod").arg("+x").arg(post_build_path))?;

        for input in all_inputs {
          for target_system in ["x86_64-linux", "x86_64-darwin"] {
            let worktree_dir = scm_dir.join(&build_tag);
            let store_path = logger.output(
              guess_build_command(&input.path)
                .args(["--argstr", "system", target_system])
                .arg("--keep-going")
                .env_clear()
                .env("NIX_BUILD_SHELL", &cfg.build_shell)
                .env("GIT_SSH_COMMAND", &git_ssh_cmd)
                .env("HOME", nix_superconf_dir.path())
                .current_dir(&worktree_dir),
            )?;

            if !store_path.status.success() {
              logger.log(format!("build exited with status {}", store_path.status))?;
              status!(BuildStatus::Failed, &finalizer_conn);
              return;
            }
            let output_path = String::from_utf8_lossy(&store_path.stdout)
              .trim_end()
              .to_string();

            sqlx::query!(
              "INSERT INTO outputs (input_id, system, store_path) VALUES ($1, $2, $3)",
              input.id,
              target_system,
              output_path
            )
            .execute(&finalizer_conn)
            .await?;
          }
        }

        if !logger
          .exec(
            Command::new("git")
              .args(["worktree", "remove", "--force"])
              .arg(&build_tag)
              .current_dir(&scm_dir),
          )?
          .success()
        {
          status!(BuildStatus::Failed, &finalizer_conn);
          return;
        };

        if !logger
          .exec(
            Command::new("rm")
              .args(["-rf"])
              .arg(scm_dir.join(&build_tag)),
          )?
          .success()
        {
          status!(BuildStatus::Failed, &finalizer_conn);
          return;
        };

        logger.exec(Command::new("echo").arg("Success!"))?;

        sqlx::query!(
          "UPDATE builds SET status = $2, finished_at = $3 WHERE id = $1",
          build_info.id,
          BuildStatus::Succeeded as _,
          Utc::now()
        )
        .execute(&finalizer_conn)
        .await?;
      };
      if let Err(e) = build_err {
        sqlx::query!(
          "UPDATE builds SET status = $1, error_msg = $2",
          BuildStatus::Failed as _,
          format!("{:?}", e)
        )
        .execute(&finalizer_conn)
        .await
        .expect("unable to update build status, everything is broken");
      }
    });
    self.jobs.insert(bid, jh);
    info!("spawned build");
    Ok(())
  }

  fn setup_secrets(upload_config: &Publish, build_hook: &mut File) -> Result<()> {
    match upload_config {
      Publish::None => scripts::None.write_into(build_hook)?,
      Publish::S3 {
        bucket,
        region,
        access_key,
        secret_key,
        nix_signing_key,
      } => {
        let mut signing_key_path = tempfile::NamedTempFile::new()?;
        write!(signing_key_path, "{nix_signing_key}")?;

        let cache_uri = format!(
          "s3://{bucket}?region={region}&secret-key={key_path}&write-nar-listing=1&\
           ls-compression=br&log-compression=br&parallel-compression=1",
          key_path = signing_key_path.path().display()
        );

        let post_build_script = scripts::S3 {
          key: access_key,
          secret: secret_key,
          cache_uri: &cache_uri,
        };

        post_build_script.write_into(build_hook)?;
      }
    }
    Ok(())
  }
}

// best effort to figure out what command will "build" the "thing".
//
// nix-shell's job is to realize the contents of environment variables too, but
// they often don't show up in $out, which results in cache false positives on
// later builds if some path happens to get GC'd
fn guess_build_command(input_path: &str) -> Command {
  if input_path.ends_with("shell.nix") {
    let mut c = Command::new("nix-shell");
    c.arg(input_path).args(["--run", "echo $out"]);
    c
  } else {
    let mut c = Command::new("nix-build");
    c.arg(input_path);
    c
  }
}

#[inline]
fn is_commit_hash(h: &str) -> bool {
  h.len() == 40 && h.bytes().all(|c| c.is_ascii_hexdigit())
}

#[derive(Template)]
#[template(path = "nix.conf", escape = "none")]
struct NixConf<'a> {
  min_free_bytes: u64,
  max_free_bytes: u64,
  builders: String,
  post_build_hook: Display<'a>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  common::init_logger();

  let _status = Command::new("git")
    .arg("--version")
    .status()
    .context("Unable to locate git. Exiting.")?;

  let cfg = common::load_config::<Config>(common::Component::Worker)?;
  let pool = PgPool::connect(&cfg.database_url)
    .await
    .with_context(|| format!("Error connecting to '{}'", cfg.database_url))?;

  sqlx::migrate!("../migrations").run(&pool).await?;

  Worker::new(cfg, &pool)?.run().await
}
