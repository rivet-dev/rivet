# Internal Dashboards

**You must have Cloudflare Access configured in `dns.cloudflare.access` namespace config.**

Exposed tunnels & applications are configured [here](/lib/bolt/core/src/dep/terraform/pools.rs).

Replace `MAIN_DOMAIN` with the value of `dns.domain.main`.

- [Consul](https://consul.MAIN_DOMAIN))
- [Nomad](https://nomad.MAIN_DOMAIN))
- [Cockroach](https://cockroach-http.MAIN_DOMAIN))
- [ClickHouse](https://clickhouse-http.MAIN_DOMAIN))
- [Prometheus (svc)](https://prometheus-svc.MAIN_DOMAIN))
- [Prometheus (job)](https://prometheus-job.MAIN_DOMAIN))
- [Minio](https://minio-console.MAIN_DOMAIN))
- [Traefik (proxied)](https://ing-px.MAIN_DOMAIN))
- [Traefik (job)](https://ing-job.MAIN_DOMAIN))
    - This does not support regional dashboards at the moment (SVC-2584)
    - Will choose a random region until fixed
