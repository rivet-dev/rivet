#!/usr/bin/env bash
# SDK comparison benchmark: the same `counter` actor served by the Rust SDK, the Java
# SDK (this repo) and the TypeScript SDK (napi runtime), driven by one Rust client
# (rivetkit-ffi/examples/bench_driver.rs).
#
# Usage (from rivetkit-java/): mise run bench:compare [seq_calls] [actors] [calls_per_actor] [rounds]
# Each SDK run gets its own fresh engine (empty storage); run order rotates per round.
# Builds: SDK sides in release; the engine is whatever engine-process resolves
# (usually target/debug/rivet-engine), shared by all runs.
set -euo pipefail

SEQ=${1:-3000}
ACTORS=${2:-20}
CALLS=${3:-100}
ROUNDS=${4:-3}
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
JAVA_DIR="$ROOT/rivetkit-java"
OUT=$(mktemp -d)
export RIVET_ENDPOINT=http://127.0.0.1:6420 RIVET_TOKEN=default RIVET_NAMESPACE=default
RUN_ID=$(date +%s)
PIDS=()

cleanup() {
  for pid in "${PIDS[@]}"; do kill "$pid" 2>/dev/null || true; done
  wait 2>/dev/null || true
}
trap cleanup EXIT

wait_for() { # file pattern seconds
  for _ in $(seq 1 $(( $3 * 10 ))); do
    grep -q "$2" "$1" 2>/dev/null && return 0
    sleep 0.1
  done
  echo "timed out waiting for '$2' in $1:"; tail -20 "$1"; return 1
}

echo "== building (release)"
cargo build --release -q -p rivetkit-ffi --lib --examples --manifest-path "$ROOT/Cargo.toml"
(cd "$JAVA_DIR" && gradle -q -Privet.cargoProfile=release :examples:installDist)
NAPI="$ROOT/rivetkit-typescript/packages/rivetkit-napi"
ls "$NAPI"/rivetkit-napi.*.node >/dev/null 2>&1 || {
  echo "missing release napi addon: run 'node scripts/build.mjs --release' in $NAPI"; exit 1; }
[ -f "$ROOT/rivetkit-typescript/packages/rivetkit/dist/tsup/mod.js" ] || {
  echo "missing TS SDK build: pnpm --filter rivetkit build"; exit 1; }
BIN="$ROOT/target/release/examples"

port_open() { [ "$(curl -s -o /dev/null -w '%{http_code}' "$RIVET_ENDPOINT/")" != "000" ]; }

# One SDK run against a fresh engine with empty storage, so neither earlier runs'
# data nor run order affects the result.
run() { # round, label, command...
  local round=$1 label=$2; shift 2
  local tag="$label-r$round"
  local storage; storage=$(mktemp -d)
  echo "== round $round: $label"
  for _ in $(seq 1 300); do port_open || break; sleep 0.1; done
  RIVETKIT_STORAGE_PATH=$storage RIVET_POOL="holder-$tag-$RUN_ID" BENCH_SPAWN_ENGINE=1 \
    "$BIN/bench_rust_server" >"$OUT/engine-$tag.log" 2>&1 &
  local holder=$!
  PIDS+=($holder)
  wait_for "$OUT/engine-$tag.log" "bench server ready" 120
  for _ in $(seq 1 600); do port_open && break; sleep 0.1; done

  local pool="bench-$tag-$RUN_ID"
  RIVETKIT_STORAGE_PATH=$storage RIVET_POOL=$pool "$@" >"$OUT/$tag.log" 2>&1 &
  local pid=$!
  PIDS+=($pid)
  wait_for "$OUT/$tag.log" "bench server ready" 120
  "$BIN/bench_driver" "$label" "$pool" "$SEQ" "$ACTORS" "$CALLS" | sed "s/^RESULT /RESULT round=$round /" | tee -a "$OUT/results.txt"
  ps -o rss= -p "$pid" | awk -v l="$label" '{printf "RSS sdk=%s rss_mb=%.0f\n", l, $1/1024}' | tee -a "$OUT/results.txt"
  kill "$pid" "$holder"; wait "$pid" "$holder" 2>/dev/null || true
  # The engine is a grandchild: wait for it (matched by its storage path) to exit
  # fully before the next run starts a new one.
  for _ in $(seq 1 600); do pgrep -f "$storage" >/dev/null || break; sleep 0.1; done
  pkill -9 -f "$storage" 2>/dev/null || true
  rm -rf "$storage"
}

SDKS=(rust java typescript)
for round in $(seq 1 "$ROUNDS"); do
  for i in 0 1 2; do
    sdk=${SDKS[$(( (i + round - 1) % 3 ))]}
    case $sdk in
      rust) run "$round" rust "$BIN/bench_rust_server" ;;
      java) run "$round" java java --enable-native-access=ALL-UNNAMED \
              -cp "$JAVA_DIR/examples/build/install/examples/lib/*" dev.rivet.examples.BenchServer ;;
      typescript) run "$round" typescript node "$JAVA_DIR/bench/ts-server.mjs" ;;
    esac
  done
done

echo
echo "== mean over $ROUNDS rounds (seq=$SEQ, par=${ACTORS}x$CALLS); logs in $OUT"
printf "%-11s %-12s %10s %9s %9s %9s %8s\n" sdk scenario calls/s p50_us p90_us p99_us rss_mb
awk '
  /^RSS/ { split($2,a,"="); split($3,b,"="); if (b[2] > rss[a[2]]) rss[a[2]]=b[2]; next }
  /^RESULT/ {
    for (i=2;i<=NF;i++) { split($i,kv,"="); f[kv[1]]=kv[2] }
    k=f["sdk"] "|" f["scenario"]
    if (!(k in n)) order[++m]=k
    n[k]++; cps[k]+=f["calls_per_s"]; p50[k]+=f["p50_us"]; p90[k]+=f["p90_us"]; p99[k]+=f["p99_us"]
  }
  END { for (i=1;i<=m;i++) { k=order[i]; split(k,r,"|");
    printf "%-11s %-12s %10.0f %9.0f %9.0f %9.0f %8s\n", r[1], r[2], cps[k]/n[k], p50[k]/n[k], p90[k]/n[k], p99[k]/n[k], rss[r[1]] } }
' "$OUT/results.txt"
