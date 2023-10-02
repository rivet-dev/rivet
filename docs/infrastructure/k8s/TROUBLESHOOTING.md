# Troubleshooting

## `/bin/k3d server` CPU usage spikes to 100% of all cores

This happens when your machine runs out of memory.

K3D is not configured with a memory cap at the moment, so this is constrained by your machine.

