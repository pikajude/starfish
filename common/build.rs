fn main() {
  pkg_config::Config::new()
    .statik(true)
    .probe("libbsd")
    .unwrap();
  vergen::EmitBuilder::builder()
    .git_sha(false)
    .fail_on_error()
    .emit()
    .unwrap();
}
