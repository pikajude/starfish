fn main() {
  vergen::EmitBuilder::builder()
    .git_sha(false)
    .fail_on_error()
    .emit()
    .unwrap();
}
