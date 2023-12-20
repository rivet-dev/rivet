#!/usr/bin/env bash
# set -euf -o pipefail

JOB_RUN_ID="{{env "NOMAD_META_JOB_RUN_ID"}}"
CONTAINER_ID=$(cat "$NOMAD_ALLOC_DIR/container-id")

# Write log shipping config
VECTOR_CONFIGS="$NOMAD_ALLOC_DIR/vector"
mkdir -p $VECTOR_CONFIGS

# Parse stream metadata & add appropriate tags for insertion
cat <<EOF > "$VECTOR_CONFIGS/remap.vrl"
# Determine which stream this message came from by the custom prefix that we append
#
# See below for more details
stream = if starts_with!(.message, "O") {
	0
} else if starts_with!(.message, "E") {
	1
} else {
	log("Unknown event stream", level: "warn", rate_limit_secs: 60)
	abort
}

# Cap line length to 1024 for data saving purposes
#
# Strip the first character, since this is used purely for metadata
message = slice!(.message, start: 1, end: 1025)

# Convert to nanoseconds for ClickHouse
ts = to_unix_timestamp(parse_timestamp!(.timestamp, format: "%+"), unit: "nanoseconds")

. = {
	"source": "job_run",
	"run_id": "${JOB_RUN_ID}",
	"task": "${NOMAD_TASK_NAME}",
	"stream_type": stream,
	"ts": ts,
	"message": message,
}
EOF

# Write config that ships logs from stdin
cat <<EOF > "$VECTOR_CONFIGS/vector.toml"
[sources.stdin]
type = "stdin"

# Reduces logging spikes. This logging is in place in order to ensure that a single
# spike of logs does not exhaust the long rate limit.
[transforms.throttle_short]
type = "throttle"
inputs = ["stdin"]
threshold = 960  # 64 logs/s
window_secs = 15

# Reduces logs from noisy games. Set reasonable caps on how
# much can be logged per minute. This is here to prevent games
# that log as fast as possible (i.e. positions of objects every
# tick) from exhausting the system while still allowing sane
# amounts of logging. This happens very frequently.
[transforms.throttle_long]
type = "throttle"
inputs = ["throttle_short"]
threshold = 1200  # 4 logs/s * 1024 bytes/log = 4096 bytes/lobby/s = 14.7 MB/lobby/hr = 353.8 MB/lobby/day  = 10.6 GB/lobby/month
window_secs = 300

[transforms.tag]
type = "remap"
inputs = ["throttle_long"]
drop_on_abort = true
file = "${VECTOR_CONFIGS}/remap.vrl"
metric_tag_values = "single"

[sinks.vector]
type = "vector"
inputs = ["tag"]
address = "127.0.0.1:5002"
healthcheck.enabled = false
compression = true
# Use less memory when buffering
buffer.max_events = 100
# Speed up for realtime logs
batch.timeout_secs = 0.25

[sinks.out]
type = "console"
inputs = ["tag"]
encoding.codec = "text"
EOF

# Run container
#
# This will prefix stdout with "O" and stderr with "E" so that we can determine the
# stream type from stdin in Vector. See the `remap.vrl` script above that determines
# the stream type and removes the prefix.
#
# Use `sleep 1` because a command that exits immediately will not be able to flush its entire
# logs to Vector and no logs will get shipped.
#
# The exit code must be preserved in order to pass the exit code to Nomad.
#
# Use `set +e` so a failiure in `runc run` doesn't immediately terminate `vector`.
set +e
{
   (runc run $CONTAINER_ID -b "$NOMAD_ALLOC_DIR/oci-bundle"; runc_exit_status=$?; sleep 1) \
   2> >(stdbuf -o0 sed 's/^/E/' >&1) \
   > >(stdbuf -o0 sed 's/^/O/' >&1)
} | /usr/bin/vector --config "$VECTOR_CONFIGS/vector.toml"
exit $runc_exit_status

