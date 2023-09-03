To run starfish in development mode without Docker, you will need to install some things first:

1. `nix` - see the installation instructions at https://nixos.org/download
2. `cargo-watch` - `cargo install cargo-watch`
3. Packages from your preferred package manager:
  - `postgresql`
  - `foreman`
  - `npm`

Then you need to create a local Postgres database called `starfish`.

Once all these parts are in place, use:

```
$ foreman start
```

The JS frontend uses Typescript and [Preact](https://preactjs.com/).

To test the Docker images, use:

```
$ docker compose build
$ docker compose up
```
