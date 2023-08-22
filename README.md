# starfish

starfish is a continuous build system that uses [Nix](https://nixos.org). It is intended to be a very lightweight, easy to configure, less featured alternative to [Hydra](https://nixos.org/hydra).

### What it does

starfish accepts build requests via a web interface and an API endpoint, attempts to build the given Nix expression(s), and reports whether the build succeeded. The web interface displays a live-updating build log.

### What it doesn't do

starfish does not handle authentication at all. Anyone with access can submit build requests. You are advised to place it behind a reverse proxy that handles authentication if you need it.

starfish is push-only; it does not monitor repositories and build new commits when they are pushed, as Hydra does.

starfish does not (currently) support sending build result notifications using services like Github webhooks.

## Installation

The easiest way to run a starfish instance is to use the prebuilt images along with Docker Compose.

```yaml
version: "3.8"

services:
  postgres:
    image: postgres:latest
    environment:
      POSTGRES_DB: starfish
      POSTGRES_USER: starfish
      POSTGRES_PASSWORD: starfish
  web:
    image: pikajude/starfish-web:latest
    links:
      - postgres
    volumes:
      - /path/to/shared/logs:/var/log/starfish
      - /path/to/config/root:/config
    ports:
      - "8000:8000"
  worker:
    image: pikajude/starfish-worker:latest
    links:
      - postgres
    volumes:
      - /path/to/shared/logs:/var/log/starfish
      - /path/to/config/root:/config
```

Note that the directory containing build logs should be shared between both containers, otherwise the web interface will be unable to live-display build logs.

## Configuration

When first launched, starfish will create default configuration files if they do not already exist. See [`config/web.default.toml`](config/web.default.toml) and [`config/worker.default.toml`](config/worker.default.toml).

Config values can be overridden using environment variables prefixed with `STARFISH.`. Examples:

- `STARFISH.LOG_PATH="/path/to/log/dir"`
- `STARFISH.PUBLISH.ACCESS_KEY="my-s3-access-key"`

In addition, there are environment-only config variables:

- `STARFISH_CONFIG_DIR` (default = `/config`): Where starfish looks for config files: `web.toml` for the web server, `worker.toml` for the worker process.
- `STARFISH_LOG` (default = `info`): Log filter. Example values: `debug`, `info`, `warn`, `error`.

  For more information on `env-logger` filters, see [the README](https://github.com/rust-cli/env_logger/blob/main/README.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md)
