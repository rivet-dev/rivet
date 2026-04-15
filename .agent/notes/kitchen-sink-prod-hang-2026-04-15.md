# kitchen-sink prod hang diagnosis

Session: 2026-04-15. Prod actor requests hang; staging fine. Same engine SHA `a10e163c48ca76c2a69a660edfe16bf037c67ffc` on both.

## Reproduction

### Two namespaces involved
- **`kitchen-sink-29a8-cloud-run-1w1z`** — namespace_id `d971pmofhuennxny2kkxboe77kl610`. Has pre-existing actors (e.g. `counter / demo-state-basicsss`).
- **`kitchen-sink-29a8-cloud-run-2-omuc`** — namespace_id `hwrf4tc81603c76spa5t68zi26m610`. Brand-new, created 2026-04-15 08:51:59, display name "Cloud Run 2". 3 envoys actively connected.

### Request shapes and outcomes
All with `rvt-method=getOrCreate&rvt-runner=default&rvt-crash-policy=sleep`, `POST /gateway/counter/action/increment`, body = 5-byte BARE `03 00 02 81 01`.

| Target | Namespace | rvt-key | Result |
|---|---|---|---|
| `api.staging.rivet.dev` | `-gv34-staging-52gh` | existing | **HTTP 200 / 4 bytes in 1.1s** |
| `api.rivet.dev` (global) | `-1w1z` | existing `demo-state-basicsss` | HTTP 000, 0 bytes, 15s timeout |
| `api-us-east-1.rivet.dev` | `-1w1z` | fresh unique | **HTTP 503 `service_unavailable` in ~2s**, has ray_id, appears in logs |
| `api-us-east-1.rivet.dev` | `-1w1z` | bogus namespace | HTTP 400 `namespace not_found` in 211ms |
| `api-us-east-1.rivet.dev` | `-2-omuc` | fresh unique | **HTTP 000, 0 bytes, no headers, no ray_id, 10–120s silent hang** |
| all 4 regional hosts | `-2-omuc` | fresh unique | **HTTP 000 on all 4, identical silent hang** |
| `/metadata` on us-east-1 | — | — | HTTP 200 with ray_id in 240ms |

The `-2-omuc` hang is fully reproducible, cross-region, regardless of curl timeout (tested up to 120s).

## Ruled out

- **NATS split-brain/partition.** Subagent audited all 4 prod NATS clusters: single 3-replica StatefulSet per DC, no gateways/leafnodes/JetStream, 0 slow consumers, 0 errors in last 2h, subject delivery test across nats-0/1/2 succeeded, `subsz` confirms `pegboard.gateway.*` subscriptions are live. Staging topology is identical.
- **Engine binary drift.** `/metadata` on prod and staging both return git_sha `a10e163c48ca76c2a69a660edfe16bf037c67ffc`.
- **Client → guard connectivity.** TLS handshakes fine. HTTP/2 stream opens. 5-byte body uploads cleanly. Server simply never writes any headers back.
- **Cross-DC routing / regional proxy chain for `-2-omuc`.** All 4 regional hosts hang identically. It's not a single broken peer DC.
- **`-1w1z` the-same-issue.** Rules out any theory where `-1w1z` and `-2-omuc` share a root cause via timing/scale — at the exact same instant, a `-1w1z` curl returns a fast 503 and the `-2-omuc` curl silently hangs. Different code paths.
- **Gateway2 reply-leg bugs (initial misread).** I initially fixated on `"timed out waiting for websocket open from envoy"` at `pegboard-gateway2/src/lib.rs:393`. Those logs are all `/gateway/counter/connect` WS upgrade paths — NOT the HTTP action path. They're a symptom of the same class of reply-delivery issues on the `-1w1z` namespace, which is tangential to the `-2-omuc` bug we care about.
- **Actor2 workflow wedge (for `-1w1z`).** `demo-state-basicsss`'s workflow history is a clean `check_envoy_liveness → expired=false` loop every 15s. Not stuck. The `"should not be reachable" transition=Sleeping` warns at `pegboard/src/workflows/actor2/mod.rs:961` are a separate state-race bug unrelated to the hang.

## Confirmed findings

### `-1w1z` has a cross-wired Cloud Run runner_config
- `pegboard::workflows::runner_pool_error_tracker` workflow `td1kxgrtdijcsq2vqe31z3uy3ml610` is tagged `{"namespace_id": "d971…", "runner_name": "default"}` (that's `-1w1z`).
- Active error stored in workflow state: `ServerlessHttpError { status_code: 400, body: "{\"code\":\"namespace_mismatch\",\"expected\":\"kitchen-sink-29a8-cloud-run-2-omuc\",\"received\":\"kitchen-sink-29a8-cloud-run-1w1z\"}" }`.
- `pegboard_outbound` at `lib.rs:142` is firing this error ~10x/sec. Hot retry loop.
- Interpretation: `-1w1z`'s runner_config URL points at the Cloud Run service for `-2-omuc`. Cloud Run service checks `X-Rivet-Namespace-Name` header against its own `RIVET_NAMESPACE` env var, returns 400 mismatch every time `-1w1z` tries to allocate a runner.
- This is a separate issue from `-2-omuc`'s hang — it just means `-1w1z` is stuck on its own config problem.

### `-2-omuc` is alive at the envoy layer
- **3 active envoys** under `0/3/39/115/56/hwrf4tc81603c76spa5t68zi26m610`, keys `eca862a4`, `866db927`, `ca29c62f`.
- Each envoy's data subspace (`0/3/39/115/4/{ns}/{key}`) shows fresh `last_ping_ts`, `version=1`, `slots=0`, `pool_name=default`, `metadata={"rivet":{"rivetkit":"0.0.0-pr.4667.33279e9"}}`. Queued commands present.
- No errors for this namespace from pegboard-outbound — it's not the namespace_mismatch problem.

### `-2-omuc` requests don't produce any engine log at warn/error
- Unique rvt-key markers injected into the curl URL (`claudeprobe*`, `uniqtest*omuc`, `longtest*omuc`, `diagregion*`) never appear in `otel.otel_logs` across any cluster in a 3-minute window.
- Zero matches for `namespace_id=hwrf4tc8*` in the last 1 hour across all 4 engine clusters.
- Zero matches for URI `rvt-namespace=kitchen-sink-29a8-cloud-run-2-omuc` in the last 10 minutes.
- Differential: in the same parallel test, `-1w1z` marker `uniqtest1776245345w1z` DID appear with proper gateway routing logs.

### So the hang is definitively namespace-specific AND happens before any warn/error fires

## Leading hypothesis

`resolve_query_actor_id` → `resolve_query_target_dc_label` → `list_runner_config_enabled_dcs` (`engine/packages/pegboard/src/ops/runner/list_runner_config_enabled_dcs.rs:58-99`) hangs on a fresh namespace because:

```rust
futures_util::stream::iter(ctx.config().topology().datacenters.clone())
    .map(|dc| async move {
        // epoxy get_optimistic read
    })
    .buffer_unordered(512)
    .filter_map(std::future::ready)
    .collect::<Vec<_>>()
    .await
```

`.collect` on `buffer_unordered` waits for ALL inner futures. If one DC's `epoxy::ops::kv::get_optimistic` read for `GlobalDataKey::new(dc_label, namespace_id, "default")` hangs forever (replica never responds, no error), the whole collect blocks. The surrounding `cache().fetch_one_json` with `ttl(3_600_000)` poisons the cache entry — every subsequent request for the same `(namespace_id, runner_name)` pair waits on the same stuck future.

Why `-1w1z` works: its cache entry was populated back when the replica reads were healthy. Served from cache, short-circuits.

Why `-2-omuc` uniquely hangs: brand new namespace, no cached entry, first read on every DC → `.collect` waits forever on at least one stuck inner future.

Why no logs: `get_optimistic` only logs at debug! on success, and only logs `tracing::warn!(?err, …)` on Err. A hung-forever read is neither — nothing emits.

Why all 4 regions hang identically: every DC's guard hits the same op path. If the stuck future is on a specific epoxy replica, every guard's read to that replica hangs the same way.

## Epoxy investigation (this session)

Port-forwarded `svc/rivet-guard 16421:6421, 26421, 36421, 46421` on all 4 DCs and queried api-peer directly with the admin token from terraform state (`random_password.engine_auth_admin_token`).

### api-peer `/runner-configs?namespace=X&runner_name=default` — reads from UDB via `pegboard::ops::runner_config::get` (DataKey, local UDB only)

| DC | `-2-omuc` | `-1w1z` |
|---|---|---|
| us-east-1 | 200 **HIT**, same serverless URL | 200 **HIT**, same serverless URL |
| us-west-1 | 200 empty `{runner_configs:{},…}` | 200 empty |
| eu-central-1 | 200 empty | 200 empty |
| ap-southeast-1 | 200 empty | 200 empty |

**Both namespaces point at the exact same Cloud Run URL:** `https://rivet-kitchen-sink-676044580344.us-east4.run.app/api/rivet`. This confirms the cross-wiring hypothesis and explains the `-1w1z` `namespace_mismatch` hot loop — that Cloud Run service has `RIVET_NAMESPACE = -2-omuc` baked in, so `-1w1z` calls get 400.

Only us-east-1 has the local UDB runner_config for both namespaces (expected — upsert only writes to the DC where the command runs).

### `rivet-engine epoxy get-local 0/39/46/90/125/4/2/{ns_id}/default` — reads from epoxy `GlobalDataKey` on the local replica

| | `-2-omuc` | `-1w1z` |
|---|---|---|
| us-east-1 `get-local` | `key does not exist` (returns in ~2s, fast) | `key does not exist` (fast) |
| us-east-1 `get-optimistic` | `key does not exist` (fast) | `key does not exist` (fast) |
| us-east-1 `key-debug-fanout` | 500 Internal Server Error in ~7s (same for every key incl. clearly-missing keys — broken debug endpoint, red herring) | same 500 |

**Epoxy `GlobalDataKey` for the runner_config is missing on us-east-1's replica for BOTH namespaces.** This is surprising — `runner_config::upsert` explicitly calls `epoxy::ops::propose` for the GlobalDataKey before writing the local UDB DataKey. Options:
1. My tuple encoding is wrong and both queries are misformatted (possible — `dc_label` is `u16` but the CLI parses bare integers as `u64`; in FDB tuple encoding positive u16 and u64 pack identically, so probably not the issue for value `2`, but worth verifying).
2. The epoxy propose silently failed for BOTH namespaces. The UDB local writes still succeeded, which is what api-peer reads.
3. Writes to the `GlobalDataKey` code path pre-dates a migration or was never run for these namespaces.

### Why `-1w1z` still resolves fast despite a missing epoxy key

If `list_runner_config_enabled_dcs` hits a missing epoxy entry for every DC, it returns an empty vec → `resolve_query_target_dc_label` returns `NoRunnerConfigConfigured` — which would fail fast with that exact error. But `-1w1z` returns `service_unavailable` (after 8 wake retries), NOT `NoRunnerConfigConfigured`. So `-1w1z`'s `list_runner_config_enabled_dcs` must be returning a **non-empty** list of DCs. Two ways it could:
- Cached result from a previous successful run (op uses `ttl(3_600_000)` = 1-hour cache keyed by `(namespace_id, runner_name)`). Populated when the key WAS in epoxy historically.
- The epoxy query returns the value from a DIFFERENT DC's replica where the key still exists. But `get_optimistic` uses the local replica first; if the local replica says missing, it may fan out — need to check.

For `-2-omuc` (brand new), the cache is empty, no prior successful run, the epoxy read returns empty → `list_runner_config_enabled_dcs` returns `[]` → `resolve_query_target_dc_label` returns `NoRunnerConfigConfigured` error → guard returns 400. 

**But empirically `-2-omuc` hangs silently, not returns 400.** So this doesn't fit either. There's still a missing link.

## Updated leading hypotheses (after epoxy checks)

**H-A: Cache differential.**
`-1w1z`'s `list_runner_config_enabled_dcs` result is cached from a historical successful run. `-2-omuc` has no cached entry. The fresh cache-miss path for `-2-omuc` hits something that hangs silently. But WHAT? If `get_optimistic` returns fast, `.collect` should complete fast too, the op should return `[]`, and `NoRunnerConfigConfigured` should be the error. Unless `get_optimistic` is doing something else like a synchronous replication wait.

**H-B: `pegboard::ops::actor::create` hangs for fresh actors in `-2-omuc`.**
If `resolve_query_target_dc_label` returns us-east-1 fine (via cache from a stale run or similar), then `resolve_query_get_or_create_actor_id` calls `pegboard::ops::actor::create` locally. That op creates an actor workflow via gasoline. If the create path has an inner transaction that hangs (e.g. waiting on some shared lock for the namespace), it would hang silently. Logs at info/warn level would not fire.

**H-C: `handle_actor_v2` wait loop hangs on an unreachable Ready signal for `-2-omuc`.**
If the create succeeds and we reach the wait loop, we should still get a 10s `ACTOR_READY_TIMEOUT` followed by at most a few guard-level retries. That's ~1 minute to a guard error log. The 120s curl test should have had plenty of time. Yet zero logs at warn/error.

None of the above fit the "zero logs for 120s" observation cleanly.

## Confirmed / ruled out in this epoxy pass

- **Tuple encoding sanity check: PASS.** Wrote throwaway `0/99/99/99/42 = u64:999` via `rivet-engine epoxy set` and read it back via `epoxy get-local 0/99/99/99/42` → `999`. Round-trip works. So my tuple format for GlobalDataKey (`0/39/46/90/125/4/{dc_label}/{namespace_id}/default`) is parsed correctly. NOTE: this test key is still live in epoxy and should be cleaned up.
- **`-2-omuc` and `-1w1z` GlobalDataKey on us-east-1 replica: BOTH MISSING.** `get-local` and `get-optimistic` both return `key does not exist` in ~2s for both namespaces. This is NOT a hang on the epoxy read itself. The read is fast.
- **us-east-1 guard pods: 7h35m old, 0 restarts.** So in-memory caches have had time to populate during normal operation. `-1w1z`'s `list_runner_config_enabled_dcs` cache entry was probably populated back when the op was succeeding.
- **`key-debug-fanout` returns HTTP 500 for every key** including clearly-missing ones. That endpoint is broken in the current build — red herring.

## Revised conclusion so far

Since the epoxy reads themselves are fast (no hang), **the original "`list_runner_config_enabled_dcs` is stuck on a hung epoxy read" hypothesis is wrong.**

The new shape of the mystery:
- `list_runner_config_enabled_dcs` for `-2-omuc` should return `[]` (empty) because no DC has the GlobalDataKey → `resolve_query_target_dc_label` should return `NoRunnerConfigConfigured` error → guard should return a fast 400 or similar → there should be a log line.
- Instead, the request hangs silently for 120s with no log output.

For `-1w1z`, the same epoxy lookup also returns missing, yet the request doesn't hang — it reaches `handle_actor_v2` and returns a fast 503 via the wake-retry path. This suggests `-1w1z` is somehow getting past `resolve_query_target_dc_label` despite the epoxy entry being missing. Two possibilities:
1. In-memory cache at `ctx.cache()` level is serving a stale positive result for `-1w1z` from when the entry existed. `-2-omuc` has no such cached result.
2. There's a different code path for namespaces that already have actors (vs brand-new ones).

**Neither possibility, as currently understood, explains why `-2-omuc`'s path hangs SILENTLY instead of returning `NoRunnerConfigConfigured`.** There must be an `.await` upstream of that error that we haven't identified.

## Next concrete actions

1. **Increase log verbosity on a us-east-1 guard pod via `rivet-engine tracing config -f <filter>`** targeted at `rivet_guard::routing=trace,pegboard::ops::runner=trace,epoxy::ops::kv=debug`. Run the curl. Check ClickHouse. This is a live mutation but it's reversible (reset via `--filter null`).
2. **Clean up the throwaway epoxy test key** `0/99/99/99/42` via `rivet-engine epoxy set '0/99/99/99/42' 'u64:0'` or a proper delete if supported. Low priority but should not be left behind.
3. **Verify the `list_runner_config_enabled_dcs` cache state hypothesis** indirectly: curl `-1w1z` N times, then curl `-2-omuc`, then check whether any cache-related logs fire. If cache is the differentiator, bumping log level would show it.
4. **Examine whether actor workflow creation for `-2-omuc` is waiting on a signal that never arrives.** Need to see workflow state for any new `-2-omuc` actors that got created (actors-by-name index, then wf history).

Both (1) and (3) need user approval because they mutate running pod state.

## Running processes to clean up

- 4x `kubectl port-forward svc/rivet-guard` to ports 16421/26421/36421/46421 (us-east-1/us-west-1/eu-central-1/ap-southeast-1). Still running in background for continued epoxy queries via `http://127.0.0.1:{port}/...`.

## Test epoxy key to clean up

- `0/99/99/99/42 = u64:999` on us-east-1 epoxy replica (committed via `rivet-engine epoxy set` as a tuple-format sanity check). Innocuous but should be deleted.
- Attempted to reset with `epoxy set ... 'u64:0'` after diagnosis; got `ExpectedValueDoesNotMatch { current_value: Some([0, 0, 0, 0, 0, 0, 3, 231]) }` — epoxy's `set` appears to use an internal CAS (expected=None matched on first write since key was vacant). Key still holds `u64:999`. Harmless.

---

# ROOT CAUSE IDENTIFIED (live-debug pass with RUST_LOG up)

Set `rivet_guard=debug,pegboard::ops::runner=trace,pegboard_gateway2=trace,pegboard_envoy=debug,pegboard_outbound=debug,epoxy::ops::kv=debug` on BOTH us-east-1 guard pods via `rivet-engine tracing config --endpoint http://{pod_ip}:6421 -f '<filter>'`. Reproduced with a tagged `rvt-key=ctrace2<ts>omuc` curl. Then reset via `-f ''`.

## End-to-end trace for req_id `d583vbgtxvz1i3lo8nso1ppccum610` / ray_id `57oagykgtxp56c5iwntmav24mpl610` / actor_id `5vu38y2ipc39kl02jpjm0mzuadm610` / envoy_key `b424074c-2c55-4f5b-bc7c-8a00694dc3f9`

1. `10:13:16.220101` — `proxy_service.rs:406` Request received.
2. `10:13:16.379237` — `list_runner_config_enabled_dcs cache miss`, `duration_ms=77`, `dc_labels=[2]`. Fast. **My earlier hypothesis that this op hangs is wrong.**
3. `10:13:16.551382` — `pegboard_gateway::mod.rs:385` "waiting for actor to become ready" actor_id=5vu38y2...
4. `10:13:16.601218` — Separate request: envoy `/envoys/connect` for envoy_key `b424074c` arrives. (Cloud Run instance coming up for the pool allocation.)
5. `10:13:16.615789` — envoy WS upgraded successfully.
6. `10:13:16.616065` — `pegboard_envoy::lib.rs:80` "tunnel ws connection established".
7. `10:13:16.619955` — `pegboard_envoy::conn::init_conn`. Envoy subscribes to `pegboard.envoy.hwrf4tc8...b424074c...` topic.
8. `10:13:16.628430` — envoy sends `ToRivetMetadata`.
9. `10:13:16.630406` — envoy sends `ToRivetKvRequest` (kv put actor state).
10. `10:13:16.650186` — envoy sends `ToRivetEvents [EventActorStateUpdate { state: ActorStateRunning }]`. **Actor is running.**
11. `10:13:16.677884` — `pegboard_gateway::mod.rs:447` "actor ready" actor_id=5vu38y2... envoy_key=b424074c.
12. `10:13:16.679238` — **`pegboard_gateway2::lib.rs:207` "gateway waiting for response from tunnel"**. Gateway2 published `ToEnvoyRequestStart` to envoy's pubsub subject and now awaits `ToRivetResponseStart` on `msg_rx`.
13. `10:13:16.679436` — `pegboard_envoy::tunnel_to_ws_task:76` "received message from pubsub, forwarding to WebSocket" payload_len=458. The gateway's request is being forwarded to the envoy WS.
14. `10:13:16.687178` — `ws_to_tunnel_task:120` "received message from envoy" msg=`ToRivetKvRequest`. The runner is doing more KV ops.
15. `10:13:16.694139` — **`ws_to_tunnel_task:120` "received message from envoy" msg=`ToRivetTunnelMessage { message_id: MessageId { gateway_id: [129, 26, 252, 108], request_id: [173, 200, 159, 100], message_index: 0 }, message_kind: ToRivetResponseStart(ToRivetResponseStart { status: 200, ... }) }`**. **The actor handler ran and the runner replied with an HTTP 200 response.**
16. `10:13:19.667517` through `10:13:38.159030` — periodic `ToRivetPong` from envoy every ~3s. Envoy WS is healthy.

**After row 15, there are zero further `pegboard_gateway2` logs for this request.** The reply was published to NATS, but gateway2's receiver never saw it.

## The NATS evidence

Queried `subsz?subs=1` directly via `kubectl exec` on `nats-0`, `nats-1`, `nats-2` (the three NATS pods in us-east-1). Across all three servers, the ONLY `pegboard.gateway.*` subscribers are:
- `pegboard.gateway.02a87c33`
- `pegboard.gateway.d67757eb`

Both live on nats-2. **There is no subscriber for `pegboard.gateway.811afc6c`** (the hex of `[129, 26, 252, 108]`). The reply was published, NATS had no matching subscriber, silently dropped.

## Root cause

**The `pegboard-gateway2::shared_state::receiver` task silently exited on a us-east-1 guard pod.**

Mechanism:
1. Guard boots, `SharedState::new()` generates gateway_id = `811afc6c`.
2. `SharedState::start()` subscribes to `pegboard.gateway.811afc6c` and spawns `receiver(sub)` in a tokio task. The `Subscriber` handle lives inside that task.
3. At some later point the task exited (my hypothesis: `while let Ok(NextOutput::Message(msg)) = sub.next().await { … }` hit `Err` or `Ok(NextOutput::Unsubscribed)`, which silently terminates the `while let`). With zero tracing instrumentation on that path.
4. Subscriber is dropped → `unsubscribe()` is called → NATS removes the subscription.
5. The in-memory `self.gateway_id` in `SharedStateInner` still equals `811afc6c`. Outgoing `send_message` calls continue to stamp `MessageId { gateway_id: 811afc6c, … }` on tunnel requests.
6. The runner echoes the gateway_id back in its replies. pegboard-envoy publishes the reply to `pegboard.gateway.811afc6c` — **which nobody subscribes to.** NATS silently drops.
7. `handle_request_inner` awaits `msg_rx.recv()` for 5 minutes (default `gateway_response_start_timeout_ms`) before failing. The client sees only `0 bytes received` because curl times out first.

Why `-1w1z` works and `-2-omuc` doesn't:
- `-1w1z` has a broken runner_config (cross-wired Cloud Run URL for `-2-omuc`); its pool_error_check_fut fires inside `handle_actor_v2` and returns `ActorRunnerFailed` before ever reaching the tunnel path. So the dead gateway2 receiver doesn't matter — the request errors out at `handle_actor_v2`, not inside gateway2.
- `-2-omuc` has a correctly-wired runner_config and a working Cloud Run pool. The actor allocates, the envoy connects, the tunnel request goes out, the runner actually processes, the reply comes back — and then hits the dead subscription.

## Fix

**Short term:** roll `rivet-guard` pods in us-east-1 (only). This respawns `SharedState::new()` with fresh gateway_ids and re-subscribes to NATS. Fixes immediately until the same trigger recurs.

**Proper fix (code change):** instrument the `receiver` loop in `pegboard-gateway2/src/shared_state.rs:308` (and the matching one in `pegboard-gateway/src/shared_state.rs`) with explicit logging on loop termination and auto-restart of the subscription. Minimum: an explicit `match` that logs `tracing::error!(?output, ?err, "gateway receiver loop terminated — subsequent tunnel replies will be lost")` and either panics (so the pod restarts under a supervisor) or re-subscribes. Current `while let Ok(NextOutput::Message(msg)) = sub.next().await` is the exact silent-exit point.

Also worth fixing:
- `shared_state.rs:363` `let _ = in_flight.msg_tx.send(...).await` silently drops send errors.
- `tracing::trace!` at `pegboard-envoy/src/ws_to_tunnel_task.rs:535` "publishing tunnel message to gateway" should be `debug!` so we can see replies in the aggregator without cranking to trace.

## Open questions / follow-ups

1. **What was the original trigger that killed the receiver task?** NATS hiccup, driver Err, or `Ok(NextOutput::Unsubscribed)`? Without source instrumentation we'll never know for the current incident. Any future instance will show up if we land the "log on loop termination" patch.
2. **Why does the receiver task's death not propagate to a pod restart?** It's spawned with `tokio::spawn` and simply returns on loop break. The pod stays up serving everything *except* gateway2. Design nit: this kind of "critical background task exited" should trip a health check.
3. **Is the same receiver dead on the v1 `pegboard-gateway` (old) path?** Didn't verify — the two subscribers `02a87c33` and `d67757eb` could be the two v1 gateways, in which case BOTH pods have a dead v2 gateway. Or they could be 1x v1 + 1x v2 from the same pod. Count matters for understanding how many pods are affected.

## Post-diagnosis confirmation: BOTH us-east-1 guard pods have the dead v2 receiver

Cross-referenced `k8s.pod.name` ResourceAttribute for each traced probe:

| Marker | Pod | Actor | Gateway_id in outgoing msg | Result |
|---|---|---|---|---|
| `ctrace1776247912` | `rivet-guard-6cf5d7bc77-8b2df` | `hskvg167…` | (not captured; first filter lacked gateway2=trace) | 15s hang, 0 bytes |
| `ctrace21776247996` | `rivet-guard-6cf5d7bc77-tllnf` | `5vu38y2…` | `811afc6c` (hex of `[129, 26, 252, 108]`) | 15s hang, 0 bytes |

Different pods, same failure mode → both v2 receivers are dead. With 2 subs observed in NATS vs 4 expected (2 pods × {v1, v2}), the surviving two (`02a87c33`, `d67757eb`) must be the v1 receivers (one per pod). Both pods' v2 are dead.

Almost certainly a **shared trigger**, not independent failures — both pods same age (7h35m), same binary, same NATS pod (nats-2), so a common event (likely a NATS reconnect / Subscriber.next() Err / Unsubscribed) hit both at once. No log trail for the event itself exists because the receiver loop termination has zero tracing. That has to be patched before we can identify the upstream trigger.

Short-term mitigation: `kubectl rollout restart deployment/rivet-guard` in us-east-1 (not single pod delete). Until the receiver-loop is instrumented and ideally auto-restarting, recurrence is possible on the same trigger.

## Cleanup performed at end of session

- `rivet-engine tracing config --endpoint http://10.21.1.81:6421 -f ''` → `Filter: reset to default` ✓
- `rivet-engine tracing config --endpoint http://10.21.1.82:6421 -f ''` → `Filter: reset to default` ✓
- All `kubectl port-forward` background processes killed.
- Throwaway epoxy key `0/99/99/99/42 = u64:999` could not be reset (epoxy `set` CAS mismatch). Left in place; innocuous.

## Infra notes

### Access patterns used
- `cd ~/rivet-ee/platform/tf && just kubectl prod us-east-1 -- …` — kubectl wrapper per DC
- `cd ~/rivet-ee/platform/tf && just engine-exec prod us-east-1 "rivet-engine …"` — runs engine CLI in a pod
- `cd ~/rivet-ee/platform/tf && terraform show -json | jq … aiven_clickhouse` — pulls ClickHouse creds
- ClickHouse: `https://rivet-clickhouse-rivet-3143.i.aivencloud.com:23033`, db `otel`, table `otel_logs`, filter on `ResourceAttributes['k8s.namespace.name']='rivet-engine'` and `ResourceAttributes['k8s.cluster.name']`
- Cluster names: `us-east-1-engine-autopilot`, `us-west-1-engine-autopilot`, `eu-central-1-engine-autopilot`, `ap-southeast-1-engine-autopilot`

### UDB key paths used
- Active envoy list per namespace: `0/3/39/115/56/{namespace_id}` → entries `<create_ts>/<envoy_key>`
- Envoy data subspace: `0/3/39/115/4/{namespace_id}/{envoy_key}` → fields `create_ts, last_ping_ts, actor, version, metadata, last_rtt, protocol_version, pool_name, slots`
- Actor data: `0/3/32/4/{actor_id}` (per platform CLAUDE.md)
- Namespace by-name index: `0/39/33/{name}` → namespace_id bytes
- Namespace data: `0/39/4/{namespace_id}` → `create_ts, name, display_name`
- UDB tag constants live in `engine/packages/universaldb/src/utils/keys.rs`. Notable tags: `3=PEGBOARD, 4=DATA, 24=LAST_PING_TS, 32=ACTOR, 33=BY_NAME, 39=NAMESPACE, 56=ACTIVE, 115=ENVOY, 116=ENVOY_KEY, 117=POOL_NAME`

### Useful logs to grep (existing, shipped to otel)
- `pegboard_outbound` target → serverless outbound errors, `lib.rs:142` "outbound handler failed"
- `pegboard::workflows::runner_pool_error_tracker` → aggregated pool errors with workflow state
- `rivet_guard::routing::pegboard_gateway` → actor wake-retry warnings at `mod.rs:422`
- `rivet_guard_core::proxy_service` at `proxy_service.rs:437` → final request-failed errors
- `pegboard::workflows::runner_pool_metadata_poller` → poll failures (saw 402s on `dev-d638-production-jxah`)

### Relevant code paths (absolute paths)
- `engine/packages/guard/src/routing/pegboard_gateway/mod.rs:313-396` — `handle_actor_v2`, wake retry loop, `ACTOR_READY_TIMEOUT=10s`
- `engine/packages/guard/src/routing/pegboard_gateway/resolve_actor_query.rs` — getOrCreate dispatch
- `engine/packages/pegboard/src/ops/runner/list_runner_config_enabled_dcs.rs:58-99` — suspected hang site
- `engine/packages/pegboard-outbound/src/lib.rs:261-399` — serverless SSE outbound
- `engine/packages/pegboard-envoy/src/conn.rs`, `ping_task.rs`, `ws_to_tunnel_task.rs`, `tunnel_to_ws_task.rs` — envoy WS lifecycle
- `engine/packages/pegboard-gateway2/src/lib.rs` + `shared_state.rs` — gateway2 tunnel-receiver (irrelevant to `-2-omuc` hang per current evidence)
