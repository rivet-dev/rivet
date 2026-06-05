# RivetKit Rust Counter

Preview Rust counter actor built directly with `rivetkit-core`.

## Run

Build the local engine once:

```bash
cargo build -p rivet-engine
```

In one terminal, start the actor runtime and let it spawn/reuse a local engine:

```bash
cd examples/rust-counter
RIVET_ENGINE_BINARY_PATH=../../target/debug/rivet-engine cargo run
```

In another terminal, create an actor and call it through the local engine API:

```bash
curl -s -X POST 'http://127.0.0.1:6420/namespaces' \
  -H 'authorization: Bearer dev' \
  -H 'content-type: application/json' \
  -d '{"name":"default","display_name":"Default"}'

ACTOR_ID=$(curl -s -X POST 'http://127.0.0.1:6420/actors?namespace=default' \
  -H 'authorization: Bearer dev' \
  -H 'content-type: application/json' \
  -d '{"name":"counter","runner_name_selector":"rivetkit-rust","key":null,"input":null,"datacenter":null,"crash_policy":"destroy"}' \
  | jq -r '.actor.actor_id')

curl -s -X POST "http://127.0.0.1:6420/gateway/$ACTOR_ID/action/increment" \
  -H 'x-rivet-encoding: json' \
  -H 'content-type: application/json' \
  -d '{"args":[]}'

curl -s -X POST "http://127.0.0.1:6420/gateway/$ACTOR_ID/action/get" \
  -H 'x-rivet-encoding: json' \
  -H 'content-type: application/json' \
  -d '{"args":[]}'
```

## Verify

```bash
cargo test -p rivetkit-rust-counter-example --test e2e -- --nocapture
```

The e2e test starts a temporary engine, serves the Rust registry, creates a counter actor, calls `increment` twice, and verifies `get` returns `2`.
