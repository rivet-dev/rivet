#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PACKAGE_DIR"

echo "=== Building package ===" >&2
pnpm run build >&2

RESULTS_DIR="$SCRIPT_DIR/results"
mkdir -p "$RESULTS_DIR"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

echo "" >&2
echo "=== Running WASM echo benchmark ===" >&2
npx tsx benchmarks/echo.bench.ts \
  > "$RESULTS_DIR/echo_${TIMESTAMP}.json" \
  2> >(tee "$RESULTS_DIR/echo_${TIMESTAMP}.log" >&2)

echo "" >&2
echo "=== Running memory benchmark (sleep workload) ===" >&2
# Run each batch size in a separate process for clean RSS measurements.
# RSS deltas are contaminated when multiple batch sizes run in the same process
# because the GC reclaims pages from earlier iterations.
SLEEP_BATCHES=(1 10 50 100 500)
for BATCH in "${SLEEP_BATCHES[@]}"; do
  echo "  --- sleep batch=$BATCH ---" >&2
  npx tsx --expose-gc benchmarks/memory.bench.ts --workload=sleep --batch="$BATCH" \
    > "$RESULTS_DIR/memory_sleep_batch${BATCH}_${TIMESTAMP}.json" \
    2> >(tee -a "$RESULTS_DIR/memory_sleep_${TIMESTAMP}.log" >&2)
done

echo "" >&2
echo "=== Running memory benchmark (PI SDK workload) ===" >&2
PI_BATCHES=(1 5 10 20)
for BATCH in "${PI_BATCHES[@]}"; do
  echo "  --- pi-sdk batch=$BATCH ---" >&2
  npx tsx --expose-gc benchmarks/memory.bench.ts --workload=pi-sdk --batch="$BATCH" \
    > "$RESULTS_DIR/memory_pi-sdk_batch${BATCH}_${TIMESTAMP}.json" \
    2> >(tee -a "$RESULTS_DIR/memory_pi-sdk_${TIMESTAMP}.log" >&2)
done

echo "" >&2
echo "=== Done. Results in $RESULTS_DIR ===" >&2
