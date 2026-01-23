# Development environment variables for running the Rivet engine locally.
# Source this file before running the engine: source scripts/run/dev-env.sh
#
# NOTE: When modifying these env vars, also update the env block in
# rivetkit-typescript/packages/rivetkit/src/engine-process/mod.ts to keep them in sync.

# Reduce backoff for runner recovery (in milliseconds)
export RIVET__PEGBOARD__RETRY_RESET_DURATION="100"
export RIVET__PEGBOARD__BASE_RETRY_TIMEOUT="100"
export RIVET__PEGBOARD__RESCHEDULE_BACKOFF_MAX_EXPONENT="1"

# Reduce thresholds for faster development iteration (in milliseconds)
export RIVET__PEGBOARD__RUNNER_ELIGIBLE_THRESHOLD="5000"
export RIVET__PEGBOARD__RUNNER_LOST_THRESHOLD="7000"

# Allow faster metadata polling for hot-reload in development (in milliseconds)
export RIVET__PEGBOARD__MIN_METADATA_POLL_INTERVAL="1000"

# Reduce shutdown durations for faster development iteration (in seconds)
export RIVET__RUNTIME__WORKER_SHUTDOWN_DURATION="1"
export RIVET__RUNTIME__GUARD_SHUTDOWN_DURATION="1"
export RIVET__RUNTIME__FORCE_SHUTDOWN_DURATION="2"
