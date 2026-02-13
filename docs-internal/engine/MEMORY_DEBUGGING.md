# Memory Debugging via Heaptrack

The heaptrack command accepts a binary file + args. After stopping it via ctrl c you can inspect the dump it produces with `heaptrack_gui <dump file>`.

## Running In a Deployment

Edit `universal/Dockerfile` to install heaptrack via apt and change the entrypoint to `["heaptrack"]` and command to `["/usr/bin/rivet-engine start"]`.

While the engine is still running (or after it stops), you can copy the heaptrack to local disk via `kubectl cp rivet-engine/<pod name>:heaptrack.rivet-engine.7.gz local-heaptrack.7.gz`. Finally run `heaptrack_gui local-heaptrack.7.gz`.
