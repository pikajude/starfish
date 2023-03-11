#![feature(try_blocks)]

use std::net::{IpAddr, SocketAddr};

use actix_web::http::header::Accept;
use actix_web::web::{self, Json};
use actix_web::{get, guard, App, HttpResponse, HttpServer, Responder};
use askama::Template;
use sqlx::PgPool;
use starfish::{BoxDynError, Build};

mod tail;

fn wrap<T, E: std::error::Error + 'static>(thing: Result<T, E>) -> actix_web::Result<T> {
  thing.map_err(|e| actix_web::error::ErrorInternalServerError(e))
}

fn content_type_guard<E>(ty: E) -> impl guard::Guard
where
  mime::Mime: PartialEq<E>,
{
  guard::fn_guard(move |ctx| {
    ctx
      .header::<Accept>()
      .map_or(false, |h| h.preference() == ty)
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
      version: env!("VERGEN_GIT_SEMVER"),
      sha: env!("VERGEN_GIT_SHA"),
    }
    .render()
    .unwrap(),
  )
}

#[get("/builds")]
async fn get_builds(db: web::Data<PgPool>) -> actix_web::Result<impl Responder> {
  Ok(Json(wrap(
    sqlx::query_as!(
      Build,
      "SELECT id, origin, created_at, error_msg, finished_at, rev, status as \"status: _\" FROM \
       builds ORDER BY created_at DESC LIMIT 10"
    )
    .fetch_all(&**db)
    .await,
  )?))
}

#[get("/build/{id}")]
async fn get_build(db: web::Data<PgPool>, id: web::Path<i32>) -> actix_web::Result<impl Responder> {
  let build = match wrap(Build::get(*id, &**db).await)? {
    Some(b) => b,
    None => return Ok(None),
  };

  let inputs = wrap(build.get_inputs_and_outputs(&**db).await)?;

  Ok(Some(Json(
    serde_json::json!({ "build": build, "inputs": inputs }),
  )))
}

#[actix_web::main]
async fn main() -> Result<(), BoxDynError> {
  env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

  let cfg = starfish::load_config()?;

  let pg = PgPool::connect(&cfg.database_url).await?;

  let listen_addr = SocketAddr::from((cfg.listen_address.parse::<IpAddr>()?, cfg.listen_port));

  Ok(
    HttpServer::new(move || {
      App::new()
        .service(actix_files::Files::new("/static", "dist").show_files_listing())
        .service(
          web::scope("/api")
            .guard(content_type_guard("application/json"))
            .service(get_builds)
            .service(get_build),
        )
        .service(web::scope("/api").service(tail::get_build_tail))
        .route(
          "/{_:.*}",
          web::get().guard(content_type_guard("text/html")).to(index),
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
