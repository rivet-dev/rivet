#!/usr/bin/env bash
set -euf -o pipefail

JOB_RUN_ID="{{env "NOMAD_META_JOB_RUN_ID"}}"

# Write log shipping config
VECTOR_CONFIGS="$NOMAD_ALLOC_DIR/vector"
mkdir -p $VECTOR_CONFIGS

for stream in stdout stderr; do
	if [[ "$stream" == "stdout" ]]; then
		stream_idx=0
	elif [[ "$stream" == "stderr" ]]; then
		stream_idx=1
	else
		echo "Invalid stream: $stream"
		exit 1
	fi

	# Add tags for insertion
	cat <<EOF > "$VECTOR_CONFIGS/remap_${stream}.vrl"
. = {
	"source": "job_run",
	"run_id": "${JOB_RUN_ID}",
	"task": "${NOMAD_TASK_NAME}",
	"stream": 1,
	# Convert to nanoseconds for ClickHouse
	"ts": to_unix_timestamp(parse_timestamp!(.timestamp, format: "%+"), unit: "nanoseconds"),
	# Cap line length to 1024
	"message": slice!(.message, start: 0, end: 1024),
}
EOF

	# Write config that takes this stream via stdin
	cat <<EOF > "$VECTOR_CONFIGS/vector_${stream}.toml"
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
file = "${VECTOR_CONFIGS}/remap_${stream}.vrl"
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
EOF
done

# Run container
#
# We spawn two instances of Vector in order to ship stdout and stderr without writing to disk
# TODO: Look at using file descriptor collector to use a single instance of Vector
CONTAINER_ID=$(cat "$NOMAD_ALLOC_DIR/container-id")
runc run $CONTAINER_ID -b "$NOMAD_ALLOC_DIR/oci-bundle" 1> >(/usr/bin/vector --config "$VECTOR_CONFIGS/vector_stdout.toml") 2> >(/usr/bin/vector --config "$VECTOR_CONFIGS/vector_stderr.toml")

