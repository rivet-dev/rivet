CREATE INDEX ON runs (cleanup_ts) WHERE cleanup_ts IS NULL;
CREATE INDEX ON run_proxied_ports (ingress_port, proxy_protocol);

