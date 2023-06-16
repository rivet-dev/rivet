# Service Port Ranges

Nomad is configured ot allow services to expose any ports in the range 20,000-25,999. See `min_dynamic_port` and `max_dynamic_port` in `salt/nomad/files/nomad.d/client.hcl.j2`.

## Reserved Ranges

- 20_000 - 20_999: Rivet services
- 21_000 - 21_099: Prometheus
- 21_600 - 21_699: Vector
- 21_700 - 21_799: Imagor
- 21_900 - 21_999: nsfw api

