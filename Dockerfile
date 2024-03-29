FROM rust:alpine3.18 as base

RUN apk add musl-dev npm
RUN cargo install cargo-chef

WORKDIR /src
COPY rust-toolchain.toml .
RUN rustc --version

FROM base AS static
COPY *.json *.js ./
RUN npm install
COPY js js
COPY css css
COPY img img
RUN npx webpack --mode=production

FROM base AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM base AS builder
COPY --from=planner /src/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

FROM debian:buster-slim AS web
COPY --from=builder /src/target/release/starfish-web /bin
COPY --from=static /src/dist /share/starfish
ENV STARFISH_RUN_MODE=production STARFISH_CONFIG_DIR=/config STARFISH_LOG=info RUST_BACKTRACE=1 RUST_LIB_BACKTRACE=1
CMD ["/bin/starfish-web"]

FROM nixos/nix:2.17.0 AS worker
COPY --from=builder /src/target/release/starfish-worker /bin
ENV STARFISH_RUN_MODE=production STARFISH_CONFIG_DIR=/config STARFISH_LOG=info RUST_BACKTRACE=1 RUST_LIB_BACKTRACE=1
CMD ["/bin/starfish-worker"]
