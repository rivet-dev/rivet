#!/usr/bin/env bash
set -euo pipefail

# Gasoline load test runner script.
#
# This script starts a Postgres container and runs multiple worker processes
# plus a bombarder against the same database to stress test the workflow engine.
#
# Usage:
#   ./engine/packages/gasoline-load-test/run.sh [options]
#
# Options:
#   --workers N          Number of worker processes (default: 3)
#   --workflows N        Number of workflows to dispatch (default: 50)
#   --signals N          Signals per workflow (default: 10)
#   --concurrency N      Concurrent signal senders (default: 10)
#   --signal-delay-ms N  Delay between signals in ms (default: 20)
#   --keep               Keep Postgres container after test

WORKERS=3
WORKFLOWS=50
SIGNALS=10
CONCURRENCY=10
SIGNAL_DELAY_MS=20
KEEP_CONTAINER=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --workers) WORKERS="$2"; shift 2 ;;
        --workflows) WORKFLOWS="$2"; shift 2 ;;
        --signals) SIGNALS="$2"; shift 2 ;;
        --concurrency) CONCURRENCY="$2"; shift 2 ;;
        --signal-delay-ms) SIGNAL_DELAY_MS="$2"; shift 2 ;;
        --keep) KEEP_CONTAINER=true; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

echo "=== Gasoline Load Test ==="
echo "Workers:          $WORKERS"
echo "Workflows:        $WORKFLOWS"
echo "Signals/workflow: $SIGNALS"
echo "Concurrency:      $CONCURRENCY"
echo "Signal delay:     ${SIGNAL_DELAY_MS}ms"
echo ""

# Build the binary first
echo "Building gasoline-load-test..."
cargo build -p gasoline-load-test 2>&1 | tail -5
BINARY="./target/debug/gasoline-load-test"

# Generate a shared test ID
TEST_ID=$(python3 -c "import uuid; print(uuid.uuid4())")
echo "Test ID: $TEST_ID"

# Set environment
export RIVET_TEST_DATABASE=postgres
export RIVET_TEST_PUBSUB=nats
export RUST_LOG="${RUST_LOG:-info,gasoline=debug,universaldb=debug}"

# Cleanup function
WORKER_PIDS=()
BOMBARDER_PID=""

cleanup() {
    echo ""
    echo "=== Cleaning up ==="

    for pid in "${WORKER_PIDS[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            echo "Stopping worker $pid"
            kill -TERM "$pid" 2>/dev/null || true
        fi
    done

    if [[ -n "$BOMBARDER_PID" ]] && kill -0 "$BOMBARDER_PID" 2>/dev/null; then
        echo "Stopping bombarder $BOMBARDER_PID"
        kill -TERM "$BOMBARDER_PID" 2>/dev/null || true
    fi

    # Wait for processes to exit
    for pid in "${WORKER_PIDS[@]}"; do
        wait "$pid" 2>/dev/null || true
    done
    if [[ -n "$BOMBARDER_PID" ]]; then
        wait "$BOMBARDER_PID" 2>/dev/null || true
    fi

    echo "All processes stopped."
}

trap cleanup EXIT

# Run in standalone mode (single process, simpler for initial testing)
echo ""
echo "=== Running standalone load test ==="
LOG_FILE="/tmp/gasoline-load-test-${TEST_ID}.log"
echo "Log file: $LOG_FILE"

$BINARY \
    --mode standalone \
    --test-id "$TEST_ID" \
    --workflow-count "$WORKFLOWS" \
    --signals-per-workflow "$SIGNALS" \
    --signal-delay-ms "$SIGNAL_DELAY_MS" \
    --concurrency "$CONCURRENCY" \
    2>&1 | tee "$LOG_FILE"

EXIT_CODE=${PIPESTATUS[0]}

echo ""
echo "=== Test Complete ==="
echo "Exit code: $EXIT_CODE"
echo "Log file: $LOG_FILE"

if [[ $EXIT_CODE -ne 0 ]]; then
    echo ""
    echo "=== ERRORS FOUND ==="
    grep -i "error\|panic\|dead\|corrupt\|frozen\|timeout" "$LOG_FILE" | tail -50 || true
fi

exit $EXIT_CODE
