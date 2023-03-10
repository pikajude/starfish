with import <nixpkgs> {};

pkgs.mkShell {
  name = "starfish";
  buildInputs = [
    stdenv
    cargo-watch
    cargo-edit
    cargo-expand
    cargo-udeps
    docker-compose
    nodejs-16_x
    foreman
    pkgsStatic.libbsd
    pkg-config
    sqlx-cli
    jq
    rustup
    zlib
  ];
}
