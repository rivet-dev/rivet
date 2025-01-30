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

## Troubleshooting

### `Illegal instruction` during `apt-get install` on macOS

If you see this output:

```
Updating certificates in /etc/ssl/certs...
Illegal instruction
Illegal instruction
dpkg: error processing package ca-certificates (--configure):
 installed ca-certificates package post-installation script subprocess returned error exit status 132
Setting up libgssapi-krb5-2:amd64 (1.20.1-2+deb12u2) ...
Setting up libcurl4:amd64 (7.88.1-10+deb12u8) ...
Setting up curl (7.88.1-10+deb12u8) ...
Processing triggers for libc-bin (2.36-9+deb12u9) ...
Errors were encountered while processing:
 ca-certificates
E: Sub-process /usr/bin/dpkg returned an error code (1)
```

Try changing your Docker VM to _Docker VMM_. See [here](https://github.com/docker/for-mac/issues/7255#issuecomment-2567154899) for more information.

