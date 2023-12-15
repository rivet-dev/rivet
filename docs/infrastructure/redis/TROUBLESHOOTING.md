# Troubleshooting

## `Error: Pools(BuildRedis(Slot refresh error. - ResponseError: Lacks the slots >= 0))`

_Only applicable if using Redis on Kubernetes._

If your cluster is under load, sometimes Redis Cluster will fail to bootstrap.

Find the cluster having this issue and rung the following:

```
kubectl delete pods -n my-cluster-name --all
```

Validate all pod are ready with:

```
kubectl get all -n my-cluster-name
```
