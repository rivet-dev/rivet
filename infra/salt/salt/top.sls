base:
  '*':
    - common
    - node_exporter
    - sysctl
    - nix
  'G@rivet:*':
    - nebula
    - cloudflared
  'G@roles:consul-server or G@roles:consul-client':
    - dnsmasq
    - consul
  'G@roles:nomad-server or G@roles:nomad-client':
    - nomad
  'G@roles:docker or G@roles:nomad-client':
    - cni_plugins
  'G@roles:docker':
    - docker
  'G@roles:nats-server':
    - nats
  'G@roles:redis':
    - redis
  'G@roles:traffic-server':
    - traffic_server
  'G@roles:cockroach':
    - cockroach
  'G@roles:clickhouse':
    - clickhouse
  'G@roles:prometheus':
    - prometheus
  'G@roles:minio':
    - minio
  'G@roles:traefik':
    - traefik
  'G@roles:traefik-cloudflare-proxy':
    - traefik_cloudflare_proxy
  'G@roles:docker-plugin-loki':
    - docker_plugin_loki
