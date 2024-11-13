# Troubleshooting

## `/bin/k3d server` CPU usage spikes to 100% of all cores

### Situation 1: Out of memory

K3D can run at 100% CPU if out of memory. The lack of memory causes OOM for containers, which causes cascading
failures which causes load on the server.

K3D is not configured with a memory cap at the moment, so this is constrained by your machine.

### Situation 2: Heavy load from crash loops

If a lot of services are frequently crashing, the K3D server will use a lot of resources to restart those
services until the `CrashLoopBackOff` state is reached.
