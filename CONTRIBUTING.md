To run starfish in development mode without Docker, use

```
$ foreman start
```

By default, it will try to connect to a local Postgres database called `starfish` using trust authentication. If you need to change this behavior, change the `database_url` setting in the appropriate configuration file in `config/dev`, or set the environment variable `STARFISH.DATABASE_URL`.

The JS frontend uses Typescript and [Preact](https://preactjs.com/).

To test the Docker images, use:

```
$ docker compose build
$ docker compose up
```
