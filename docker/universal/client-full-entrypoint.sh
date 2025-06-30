#!/bin/bash
set -e

# Start cadvisor in the background
cadvisor \
  --port=7780 \
  --listen_ip=0.0.0.0 \
  --prometheus_endpoint="/metrics" \
  --enable_metrics=cpu,cpuLoad,memory,network,disk,diskIO,oom_event,process,tcp,udp \
  --docker_only=false \
  --disable_root_cgroup_stats=false &

# TODO:
# --raw_cgroup_prefix_whitelist="" \

# Start rivet-client with all passed arguments
exec rivet-client "$@"
