Starfish is a service that reimplements a subset of [Hydra](https://github.com/nixos/hydra). Specifically, it accepts a repo URL and git SHA and builds the paths in a nix file. This service exists because Nix path signing requires a private key on the local filesystem, which is insecure in most CI environments.

## Hacking

This project uses unstable rustc, which means if you're on stable, you will get some errors from rustfmt, because stable rustfmt doesn't have features.

Starfish requires access to a number of secrets - an AWS key pair, an SSH key, and a Nix signing key - that aren't bundled in this repo for security reasons. The actual secrets used in production are in the [Jabberwocky repo](https://gitlab.com/dfinity-lab/infra-group/jabberwocky-harbormaster).

In development mode, if you use `direnv`, you can create a `.env` file with the following contents:

```sh
export ROCKET_SIGNING_KEY=...
export ROCKET_AWS_ACCESS_KEY=...
export ROCKET_AWS_SECRET_KEY=...
# This is a base64-encoded key. You can use `cat my_key | openssl base64 -e -A` to get the encoded version of some key you have.
export ROCKET_SSH_PRIVATE_KEY=...
```

(replacing the dots with the actual values).

Starfish requires a postgres database on your system called `starfish`. One day this will be configurable. After you've created the database, do:

```
foreman start
```

Navigate to http://localhost:8000.

If you want, you can run `cargo doc` to build rustdocs; foreman serves them on `localhost:8080`.

### Hacking the worker

The worker uses postgres notifications the same way Hydra does. To test the worker, start it up, launch psql and issue

```
notify build_queued, '18';
```

and replace `18` with the build ID. The worker prints out what it's doing to stderr.

The file `sqlx-data.json` is used to build Starfish inside the docker image, where it doesn't have access to a running postgres instance. Run `cargo sqlx prepare --merged` before doing a Docker rebuild.

### Hacking the webserver

Starfish's frontend uses the Rocket framework. The guide for Rocket is [here](https://rocket.rs/v0.5-rc/guide/).

### Docker

```
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up
```

If you don't specify `-f docker-compose.yml`, docker-compose will just pull the latest built images from the Gitlab registry.
