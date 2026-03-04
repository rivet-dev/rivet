# Rivet Engine Kubernetes Quick Deployment

**For the complete deployment guide, see the [Kubernetes documentation](https://rivet.dev/docs/self-hosting/kubernetes).**

This guide is for developers who have already cloned the repository and want to deploy locally.

## Prerequisites

- Kubernetes cluster (v1.24+)
- `kubectl` configured
- Metrics server (required for HPA) - included by default in most distributions

## Deploy

```bash
# Apply all manifests
kubectl apply -f engine/

# Wait for pods
kubectl -n rivet-engine wait --for=condition=ready pod -l app=postgres --timeout=300s
kubectl -n rivet-engine wait --for=condition=ready pod -l app=rivet-engine --timeout=300s
```

## Verify

```bash
# Check pods (you should see 2+ engine pods)
kubectl -n rivet-engine get pods

# Check HPA status
kubectl -n rivet-engine get hpa

# Port forward
kubectl -n rivet-engine port-forward svc/rivet-engine 6420:6420 6421:6421

# Test health
curl http://localhost:6421/health
```

Expected response:
```json
{"runtime":"engine","status":"ok","version":"..."}
```

## Logs

```bash
# View engine logs
kubectl -n rivet-engine logs -l app=rivet-engine -f
```

## Updating Configuration

After modifying the `engine-config` ConfigMap, restart the engine pods to pick up the changes:

```bash
kubectl apply -f engine/02-engine-configmap.yaml
kubectl -n rivet-engine rollout restart deployment/rivet-engine
```

## Cleanup

```bash
kubectl delete namespace rivet-engine
```

## Next Steps

See [README.md](README.md) for more deployment options and [the documentation](https://rivet.dev/docs/self-hosting/kubernetes) for production setup.
