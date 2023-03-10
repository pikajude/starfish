fn main() {
  pkg_config::Config::new()
    .statik(true)
    .probe("libbsd")
    .unwrap();
  vergen::vergen(vergen::Config::default()).unwrap();
}
