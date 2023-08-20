#![feature(try_blocks)]

use actix_files::{Files, NamedFile};
use actix_web::http::header::Accept;
use actix_web::{get, guard, put, web, App, HttpResponse, HttpServer, Responder};
use askama::Template;
use cfg::Config;
use common::{BoxDynError, Build};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

mod cfg;
mod tail;

#[derive(Debug, Deserialize)]
struct BuildPlsNew {
  origin: String,
  rev: String,
  paths: String,
}

fn wrap<T, E: std::error::Error + 'static>(thing: Result<T, E>) -> actix_web::Result<T> {
  thing.map_err(|e| actix_web::error::ErrorInternalServerError(e))
}

fn content_type_guard<E: PartialEq<mime::Mime>>(ty: E) -> impl guard::Guard {
  guard::fn_guard(move |ctx| {
    ctx
      .header::<Accept>()
      .map_or(false, |h| ty == h.preference())
  })
}

async fn index() -> impl Responder {
  #[derive(Template)]
  #[template(path = "index.html")]
  struct IndexPage {
    version: &'static str,
    sha: &'static str,
  }

  HttpResponse::Ok().content_type(mime::TEXT_HTML_UTF_8).body(
    IndexPage {
      sha: common::STARFISH_VERSION,
      version: env!("CARGO_PKG_VERSION"),
    }
    .render()
    .unwrap(),
  )
}

#[get("/builds")]
async fn get_builds(db: web::Data<PgPool>) -> actix_web::Result<impl Responder> {
  Ok(web::Json(wrap(
    sqlx::query_as!(
      Build,
      "SELECT id, origin, created_at, error_msg, finished_at, rev, status as \"status: _\" FROM \
       builds ORDER BY created_at DESC LIMIT 10"
    )
    .fetch_all(&**db)
    .await,
  )?))
}

#[put("build")]
async fn put_build(
  db: web::Data<PgPool>,
  build: web::Json<BuildPlsNew>,
) -> actix_web::Result<impl Responder> {
  let new_build = wrap(
    sqlx::query_as!(
      Build,
      "INSERT INTO builds (origin, rev) VALUES ($1, $2) RETURNING id, origin, rev, created_at, \
       status as \"status: _\", finished_at, error_msg",
      &build.origin,
      &build.rev
    )
    .fetch_one(&**db)
    .await,
  )?;

  let mut all_paths = vec![];

  for path in build.paths.split(',') {
    let path = path.trim();
    if !path.is_empty() {
      all_paths.push(path.to_string());
    }
  }

  // extremely budget multi insert because sqlx doesn't support it
  wrap(
    sqlx::query!(
      "INSERT INTO inputs (build_id, path) SELECT $1, * FROM UNNEST($2::text[])",
      new_build.id,
      all_paths.as_slice()
    )
    .execute(&**db)
    .await,
  )?;

  wrap(
    sqlx::query!(
      "SELECT pg_notify($1, $2)",
      "build_queued",
      new_build.id.to_string()
    )
    .execute(&**db)
    .await,
  )?;

  Ok(web::Json(new_build))
}

#[get("/build/{id}")]
async fn get_build(db: web::Data<PgPool>, id: web::Path<i32>) -> actix_web::Result<impl Responder> {
  let Some(build) = wrap(Build::get(*id, &**db).await)? else {
    return Ok(None)
  };

  let inputs = wrap(build.get_inputs_and_outputs(&**db).await)?;

  Ok(Some(web::Json(json!({ "build": build, "inputs": inputs }))))
}

#[get("/build/{id}/raw")]
async fn get_build_raw(cfg: web::Data<Config>, id: web::Path<i32>) -> Option<NamedFile> {
  NamedFile::open_async(cfg.logfile(*id))
    .await
    .ok()
    .map(|x| x.set_content_type(mime::TEXT_PLAIN))
}

#[put("/build/{id}/restart")]
async fn put_build_restart(
  db: web::Data<PgPool>,
  id: web::Path<i32>,
) -> actix_web::Result<impl Responder> {
  wrap(
    sqlx::query!(
      "SELECT pg_notify($1, $2)",
      "build_restarted",
      id.to_string()
    )
    .execute(&**db)
    .await,
  )?;

  Ok(web::Json(json!({"success": true})))
}

#[actix_web::main]
async fn main() -> Result<(), BoxDynError> {
  env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

  let cfg = common::load_config::<Config>("config/web")?;

  let pg = PgPool::connect(&cfg.database_url).await?;

  let listen_addr = cfg.listen_addr()?;

  Ok(
    HttpServer::new(move || {
      App::new()
        .service(Files::new("/static", &cfg.static_root))
        .service(
          web::scope("/api")
            .guard(content_type_guard(mime::APPLICATION_JSON))
            .service(get_builds)
            .service(get_build)
            .service(put_build)
            .service(put_build_restart),
        )
        .service(web::scope("/api").service(tail::get_build_tail))
        .service(get_build_raw)
        .route(
          "/{_:.*}",
          web::get()
            .guard(content_type_guard(mime::TEXT_HTML))
            .to(index),
        )
        .app_data(web::Data::new(pg.clone()))
        .app_data(web::Data::new(cfg.clone()))
        .wrap(actix_web::middleware::Logger::default())
    })
    .bind(listen_addr)?
    .run()
    .await?,
  )
}
