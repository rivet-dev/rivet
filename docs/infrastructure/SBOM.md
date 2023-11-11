# Software Bill of Materials

## Rust packages

License policies are defined in `deny.toml`.

Run the following:

```bash
cargo deny --manifest-path svc/Cargo.toml list
```

## Infrastructure

_Loosely follows [SPDX](https://spdx.org/licenses/) format._

| Software              | License                            |
| --------------------- | ---------------------------------- |
| Apache Traffic Server | Apache-2.0                         |
| CNI                   | Apache-2.0                         |
| ClickHouse            | Apache-2.0                         |
| Cloudflared           |                                    |
| Cockroach             | Apache-2.0                         |
| Docker                | Apache-2.0                         |
| Imagor                | Apache-2.0                         |
| Minio                 | AGPL-3.0 (non-derivative, dev use) |
| NATS                  | Apache-2.0                         |
| Node Exporter         | Apache-2.0                         |
| Nomad                 | MPL-2.0                            |
| Prometheus            | Apache-2.0                         |
| Redis Exporter        | MIT                                |
| Redis                 | BSD-3                              |
| Terraform             | Apache-2.0                         |
| Traefik               | MIT                                |
| cloudflared           | Apache-2.0                         |
| curl                  | curl                               |
| go-migrate            | MIT                                |
| jq                    | MIT                                |
| nsfw_api              | Apache-2.0                         |
| open_nsfw             | Apache-2.0                         |
| rsync                 | GPL-3.0 (non-derivative)           |
