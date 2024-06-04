# Start times

## Low-end machine

> System
>
> - Debian GNU/Linux 11
> - Shared VM, 4 VCPUs (of AMD EPYC 7713 16-Core 2GHz)
> - 8GB memory

### `nix-shell` setup time (fresh)

- Before building `bolt`: 1m31s
- Building `bolt`: 2m15s

### Services (Minimal setup)

| step               | up    |
| ------------------ | ----- |
| k8s-cluster        | 20s   |
| k8s-infra          | 2m31s |
| redis              | 1s    |
| cockroach          | 1s    |
| clickhouse         | 1s    |
| s3                 | 24s   |
| infra-artifacts    | 50s   |
| migrate            | 62s   |
| up (containerized) | 7s    |
| total              | 5m17s |

### `k8s-infra` breakdown

_Note, these are not additive as they run in parallel_

_First loki, promtail, and prometheus are provisioned then the rest follow_

| service        | up    |
| -------------- | ----- |
| promtail       | 3s    |
| prometheus     | 43s   |
| loki           | 1m14s |
| k8s_dashboard  | 3s    |
| traefik tunnel | 20s   |
| traefik        | 20s   |
| traffic_server | 26s   |
| nats           | 27s   |
| imagor         | 29s   |
| minio          | 35s   |
| nomad_server   | 46s   |
| clickhouse     | 47s   |
| redis          | 51s   |
| nsfw_api       | 56s   |
| cockroachdb    | 1m6s  |

## Higher-end machine

> System
>
> - Debian GNU/Linux 11
> - AMD EPYC 7713 16-Core 2GHz
> - 32GB memory

### Services (Complex setup)

_This setup uses postgres as the terraform config storage method, adding overhead to each step_

| step               | up       | destroy  |
| ------------------ | -------- | -------- |
| k8s-cluster        | 27s      | 16s      |
| k8s-infra          | 2m34s    | -        |
| tls                | 4m29s    | 5s       |
| redis              | 11s      | -        |
| cockroach          | 10s      | -        |
| clickhouse         | 10s      | -        |
| vector             | 19s      | -        |
| pools              | 2m43s    | 1m57s    |
| dns                | 2m48s    | 9s       |
| better uptime      | untested | untested |
| cf-workers         | 15s      | 6s       |
| cf-tunnels         | 18s      | 12s      |
| s3                 | 35s      | -        |
| infra-artifacts    | 35s      | -        |
| migrate            | 58s      | -        |
| up (containerized) | 7s       | -        |
| total              | 17m2s    | 2m40s    |

### `k8s-infra` breakdown

| service        | up    |
| -------------- | ----- |
| promtail       | 6s    |
| prometheus     | 48s   |
| loki           | 1m20s |
| k8s_dashboard  | 6s    |
| imagor         | 8s    |
| traefik        | 12s   |
| traefik tunnel | 14s   |
| traffic_server | 16s   |
| minio          | 22s   |
| nats           | 28s   |
| clickhouse     | 30s   |
| redis          | 33s   |
| nsfw_api       | 36s   |
| nomad_server   | 46s   |
| cockroachdb    | 49s   |
