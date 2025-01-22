# Full Development Docker Compose

## Operating

Find our docs [here](https://rivet.gg/docs/self-hosting/docker-compose).

## Development

### Rebuilding

To rebuild all services, run:

```bash
docker compose -f docker/dev-full/docker-compose.yml up -d --build
```

To rebuild just the server, run:

```bash
docker compose -f docker/dev-full/docker-compose.yml up -d --build rivet-server
```

### Logs

To fetch logs for a service, run:

```bash
docker compose -f docker/dev-full/docker-compose.yml logs {name}
```

#### Following

To follow logs, run:

```bash
docker compose -f docker/dev-full/docker-compose.yml logs -f {name}
```

#### Grep

It's common to use grep (or the more modern
[ripgrep](https://www.google.com/search?q=ripgrep&oq=ripgrep&sourceid=chrome&ie=UTF-8)) to filter logs.

For example, to find all errors in `rivet-server` with the 10 preceding lines, run:

```bash
docker compose -f docker/dev-full/docker-compose.yml logs rivet-server | grep -B 10 level=error
```

Logs for `rivet-server` and `rivet-client` can also be configured via the environment. See [here](/docs/self-hosting/client-config) for
more information.

