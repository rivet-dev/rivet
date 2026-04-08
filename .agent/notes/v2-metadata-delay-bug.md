# Bug: v2 actor dispatch requires ~5s delay after metadata refresh

## Reproduce

```bash
# 1. Start engine with the force-v2 hack (see below)
rm -rf ~/.local/share/rivet-engine/db
cargo run --bin rivet-engine -- start

# 2. Start test-envoy
RIVET_ENDPOINT=http://127.0.0.1:6420 RIVET_TOKEN=dev RIVET_NAMESPACE=default \
  RIVET_POOL_NAME=test-envoy AUTOSTART_ENVOY=0 AUTOSTART_SERVER=1 \
  AUTOCONFIGURE_SERVERLESS=0 cargo run -p rivet-test-envoy

# 3. In another terminal, run this:
NS="repro-$(date +%s)"
curl -s -X POST -H "Authorization: Bearer dev" -H "Content-Type: application/json" \
  http://localhost:6420/namespaces -d "{\"name\":\"$NS\",\"display_name\":\"$NS\"}"
curl -s -X PUT -H "Authorization: Bearer dev" -H "Content-Type: application/json" \
  "http://localhost:6420/runner-configs/test-envoy?namespace=$NS" \
  -d '{"datacenters":{"default":{"serverless":{"url":"http://localhost:5051/api/rivet","request_lifespan":300,"max_concurrent_actors":10000,"slots_per_runner":1,"min_runners":0,"max_runners":10000}}}}'
curl -s -X POST -H "Authorization: Bearer dev" -H "Content-Type: application/json" \
  "http://localhost:6420/runner-configs/test-envoy/refresh-metadata?namespace=$NS" -d '{}'

# THIS FAILS (no delay):
curl -s -X POST -H "Authorization: Bearer dev" -H "Content-Type: application/json" \
  "http://localhost:6420/actors?namespace=$NS" \
  -d "{\"name\":\"test\",\"key\":\"k-$(date +%s)\",\"runner_name_selector\":\"test-envoy\",\"crash_policy\":\"sleep\"}" \
  | python3 -c "import json,sys; a=json.load(sys.stdin)['actor']['actor_id']; print(a)" \
  | xargs -I{} curl -s --max-time 12 -H "X-Rivet-Token: dev" -H "X-Rivet-Target: actor" -H "X-Rivet-Actor: {}" http://localhost:6420/ping
# Expected: actor_ready_timeout

# THIS WORKS (5s delay):
sleep 5
curl -s -X POST -H "Authorization: Bearer dev" -H "Content-Type: application/json" \
  "http://localhost:6420/actors?namespace=$NS" \
  -d "{\"name\":\"test\",\"key\":\"k2-$(date +%s)\",\"runner_name_selector\":\"test-envoy\",\"crash_policy\":\"sleep\"}" \
  | python3 -c "import json,sys; a=json.load(sys.stdin)['actor']['actor_id']; print(a)" \
  | xargs -I{} curl -s --max-time 12 -H "X-Rivet-Token: dev" -H "X-Rivet-Target: actor" -H "X-Rivet-Actor: {}" http://localhost:6420/ping
# Expected: 200 with JSON body
```

## Symptom

Actor is created (200), envoy receives CommandStartActor, actor starts in ~10ms, EventActorStateUpdate{Running} is sent back via WS, but the guard returns `actor_ready_timeout` after 10 seconds. The actor never becomes connectable.

## Root cause

After `refresh-metadata` stores `envoyProtocolVersion` in the DB, the runner pool workflow (`pegboard_runner_pool`) needs to restart its serverless connection cycle to use v2 POST instead of v1 GET. This takes ~2-5 seconds because:

1. The `pegboard_runner_pool_metadata_poller` workflow runs on a polling interval
2. The `pegboard_serverless_conn` workflow needs to cycle its existing connections
3. The `pegboard_runner_pool` workflow reads the updated config and spawns new v2 connections

Until this happens, the engine dispatches via v1 GET SSE which doesn't deliver the start payload to the envoy.

## Code locations

### Force-v2 hack (temporary)
`engine/packages/pegboard/src/workflows/actor/runtime.rs` line ~268:
```rust
// Changed from: if pool.and_then(|p| p.protocol_version).is_some()
// To force v2 for all serverless pools:
if pool.as_ref().and_then(|p| p.protocol_version).is_some() || for_serverless {
```

### Where protocol_version is stored
`engine/packages/pegboard/src/workflows/runner_pool_metadata_poller.rs` line ~214:
```rust
if let Some(protocol_version) = metadata.envoy_protocol_version {
    tx.write(&protocol_version_key, protocol_version)?;
}
```

### Where protocol_version is read for v1→v2 migration decision
`engine/packages/pegboard/src/workflows/actor/runtime.rs` in `allocate_actor_v2`:
```rust
let pool_res = ctx.op(crate::ops::runner_config::get::Input { ... }).await?;
// ...
if pool.and_then(|p| p.protocol_version).is_some() {
    return Ok(AllocateActorOutputV2 { status: AllocateActorStatus::MigrateToV2, ... });
}
```

### Where runner config is cached (may need invalidation)
`engine/packages/pegboard/src/ops/runner_config/get.rs` - reads ProtocolVersionKey from DB

### Where v1 (GET) vs v2 (POST) connection is made
- v1: `engine/packages/pegboard/src/workflows/serverless/conn.rs` line ~301: `client.get(endpoint_url)`
- v2: `engine/packages/pegboard-outbound/src/lib.rs` line ~316: `client.post(endpoint_url).body(payload)`

## Fix needed

After `refresh-metadata` stores `envoyProtocolVersion`, the runner pool should immediately use v2 POST without waiting for the metadata poller cycle. Either:
1. Signal the runner pool workflow to restart connections when metadata changes
2. Make the `refresh-metadata` API synchronously update the runner pool state
3. Have the serverless conn workflow check protocol_version before each connection attempt instead of relying on the metadata poller cycle
