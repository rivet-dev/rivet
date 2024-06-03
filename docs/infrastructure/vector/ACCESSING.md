# Accessing Vector

To access vector running in the k8s cluster, first forward it and then use the vector cli pointed at the forwarded http url.

## Forwarding

`./scripts/forward/vector.sh`

## Vector CLI

`nix-shell -p vector`

`vector top`
