# Troubleshooting

## `No nodes found for the given cluster`

This is the error when running a Bolt command:

```
FATA[0000] No nodes found for given cluster
thread 'main' panicked at /root/rivet/ee/oss/lib/bolt/core/src/tasks/gen.rs:26:39:
```

Run:

```
BOLT_SKIP_K8S_GEN=1 bolt tf destroy k8s_cluster_k3d
bolt tf apply k8s_cluster_k3d
```

This will tell Terraform to forget about the K3D cluster and create a new one.

## Check what ports are forwarded to my load balancer?

Check the ports forwarded with this command:

```
docker ps | grep k3d
```

## I'm getting `Empty reply from server`

This means Traefik hasn't started yet. Make sure the deployment works.

## Pods stuck in `Pending`

Run `kubectl describe` on the pod. Your machine likely ran out of storage or memory.

Once you have cleaned up disk space, run `kubectl describe nodes` to see if the node show the error still there. You may have to run `k3d cluster stop <cluster name> && k3d cluster start <cluster name>` to restart the cluster.

If you see `Unschedulable: true`, fix it by running: `kubectl uncordon <node name>`
