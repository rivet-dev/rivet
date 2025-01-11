# TLS Rotation

Edge TLS certs get pulled periodically by the systemd cron defined in `/packages/services/cluster/src/workflows/server/install/install_scripts/files/rivet_fetch_tunnel_tls.sh` (and `/packages/services/cluster/src/workflows/server/install/install_scripts/files/rivet_fetch_gg_tls.sh` for GG nodes). Certificates are downloaded from our core cluster via api-edge and overwrite existing certs at `/etc/__TRAEFIK_INSTANCE_NAME__/tls`.

## Tunnel Certificates

Tunnel certificates have a 1 year validity period and are refreshed by `cluster-tunnel-tls-renew`.

## GG Certificates

GG Certificates rotate per datacenter. Renewal is triggered by `cluster-datacenter-tls-renew` and issued by
the `cluster_datacenter_tls_issue` workflow.
