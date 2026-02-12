# Rivet Engine Kubernetes Quick Deployment

**For the complete deployment guide, see the [Kubernetes documentation](https://rivet.gg/docs/self-hosting/kubernetes).**

This guide is for developers who have already cloned the repository and want to deploy locally.

## Prerequisites

- Kubernetes cluster (v1.24+)
- `kubectl` configured
- Metrics server (required for HPA) - included by default in most distributions

## Architecture

The Rivet Engine deployment consists of two components:

- **Main Engine Deployment**: Runs all services except singleton services. Configured with Horizontal Pod Autoscaling (HPA) to automatically scale between 2-10 replicas based on CPU (60%) and memory (80%) utilization.
- **Singleton Engine Deployment**: Runs singleton services that must have exactly 1 replica (e.g., schedulers, coordinators).

## Deploy

```bash
# Apply all manifests
kubectl apply -f engine/

# Wait for pods
kubectl -n rivet-engine wait --for=condition=ready pod -l app=postgres --timeout=300s
kubectl -n rivet-engine wait --for=condition=ready pod -l app=rivet-engine --timeout=300s
kubectl -n rivet-engine wait --for=condition=ready pod -l app=rivet-engine-singleton --timeout=300s
```

## Verify

```bash
# Check pods (you should see 2+ engine pods and 1 singleton pod)
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
# View main engine logs
kubectl -n rivet-engine logs -l app=rivet-engine -f

# View singleton engine logs
kubectl -n rivet-engine logs -l app=rivet-engine-singleton -f
```

## Cleanup

```bash
kubectl delete namespace rivet-engine
```

## Next Steps

See [README.md](README.md) for more deployment options and [the documentation](https://rivet.gg/docs/self-hosting/kubernetes) for production setup.
