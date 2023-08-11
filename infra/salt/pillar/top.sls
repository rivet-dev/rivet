base:
  '*':
    - common
    - nix
  'G@rivet:*':
    - rivet
    - cloudflared
  'G@roles:consul-client or G@roles:consul-server':
    - consul
  'G@roles:nomad-client or G@roles:nomad-server':
    - nomad
  'G@roles:traffic-server':
    - s3
    - ats
  'G@roles:traefik':
    - tls
    - api-route
  'G@roles:clickhouse':
    - clickhouse
  'G@roles:minio':
    - minio
  'G@roles:cloudflare-proxy':
    - cloudflare

