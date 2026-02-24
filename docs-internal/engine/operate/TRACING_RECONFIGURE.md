# Dynamic Tracing Configuration

Dynamically reconfigure log levels and OpenTelemetry sampling for all running services without restart.

## Log Filter Configuration

Control which log messages are displayed by setting filter directives (similar to `RUST_LOG`).

**Set log filter to debug**

```bash
rivet-engine tracing config -f debug

# Or via HTTP API:
curl -X PUT http://localhost:6421/debug/tracing/config \
  -H "Content-Type: application/json" \
  -d '{"filter":"debug"}'
```

**Debug a specific package**

```bash
rivet-engine tracing config -f "debug,rivet_api_peer=trace"

# Or via HTTP API:
curl -X PUT http://localhost:6421/debug/tracing/config \
  -H "Content-Type: application/json" \
  -d '{"filter":"debug,rivet_api_peer=trace"}'
```

**Reset log filter to defaults**

```bash
rivet-engine tracing config -f ""

# Or via HTTP API:
curl -X PUT http://localhost:6421/debug/tracing/config \
  -H "Content-Type: application/json" \
  -d '{"filter":null}'
```

## OpenTelemetry Sampler Ratio

Control what percentage of traces are sampled and sent to the OpenTelemetry collector.

**Set sampler ratio to 10%**

```bash
rivet-engine tracing config -s 0.1

# Or via HTTP API:
curl -X PUT http://localhost:6421/debug/tracing/config \
  -H "Content-Type: application/json" \
  -d '{"sampler_ratio":0.1}'
```

**Set sampler ratio to 100% (capture all traces)**

```bash
rivet-engine tracing config -s 1.0

# Or via HTTP API:
curl -X PUT http://localhost:6421/debug/tracing/config \
  -H "Content-Type: application/json" \
  -d '{"sampler_ratio":1.0}'
```

**Reset sampler ratio to default**

```bash
rivet-engine tracing config -s 0.001

# Or via HTTP API:
curl -X PUT http://localhost:6421/debug/tracing/config \
  -H "Content-Type: application/json" \
  -d '{"sampler_ratio":null}'
```

