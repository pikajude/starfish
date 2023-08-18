use askama::Template;

#[derive(Template)]
#[template(path = "post-build/s3.sh", escape = "none")]
pub struct S3<'a> {
  pub key: &'a str,
  pub secret: &'a str,
  pub cache_uri: &'a str,
}

#[derive(Template)]
#[template(path = "post-build/none.sh", escape = "none")]
pub struct None;
