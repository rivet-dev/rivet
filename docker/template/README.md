# Docker Compose Template Generator

This package generates Docker Compose configurations for Rivet environments with different scaling configurations.

Run with:

```sh
pnpm start
```

## Port Management

The template uses a base port (default: 6420) and assigns sequential ports to services:

- 6420: Rivet Engine
- 5050: Runner (mapped from original)
- 4222: NATS (mapped from original)
- 3100: Grafana (mapped from original 3000)
- 9300/9301: ClickHouse (mapped from original)
- 4317/4318/8888: OpenTelemetry Collector (mapped from original)
