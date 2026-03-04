# Rivet Engine Kubernetes Deployment

Kubernetes manifests for deploying Rivet Engine.

## Quick Start

**For a complete quick start guide with copy-paste YAML manifests, see the [Kubernetes documentation](https://rivet.dev/docs/self-hosting/kubernetes).**

## What's in This Directory

The `engine/` directory contains reference Kubernetes manifests for advanced deployments:

- `00-namespace.yaml` - Namespace definition
- `01-serviceaccount.yaml` - Service account
- `02-engine-configmap.yaml` - Engine configuration
- `03-rivet-engine-deployment.yaml` - Engine deployment
- `04-rivet-engine-service.yaml` - Service definition
- `05-rivet-engine-hpa.yaml` - Horizontal Pod Autoscaler
- `06-postgres-configmap.yaml` - PostgreSQL configuration
- `07-postgres-secret.yaml` - PostgreSQL credentials
- `08-postgres-statefulset.yaml` - PostgreSQL database
- `09-postgres-service.yaml` - PostgreSQL service
- `10-rivet-engine-pdb.yaml` - Pod Disruption Budget

The default configuration uses a single datacenter named `default`. If you plan on running a multi-region deployment, update the `topology` section in `02-engine-configmap.yaml` with your datacenter names and URLs.

## Updating Configuration

After modifying the `engine-config` ConfigMap, restart the engine pods to pick up the changes:

```bash
kubectl apply -f engine/02-engine-configmap.yaml
kubectl -n rivet-engine rollout restart deployment/rivet-engine
```

## Local Development

For local development with k3d:

```bash
./scripts/run/k8s/engine.sh
```

This script creates a k3d cluster, builds the image, and deploys everything.

## Production Deployment

For production deployments, see the steps outlined in our [Kubernetes Self-Hosting Guide](https://rivet.dev/docs/self-hosting/kubernetes).
