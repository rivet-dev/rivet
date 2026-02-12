# Rivet Engine Kubernetes Deployment

Kubernetes manifests for deploying Rivet Engine.

## Quick Start

**For a complete quick start guide with copy-paste YAML manifests, see the [Kubernetes documentation](https://rivet.gg/docs/self-hosting/kubernetes).**

## What's in This Directory

The `engine/` directory contains reference Kubernetes manifests for advanced deployments:

- `00-namespace.yaml` - Namespace definition
- `01-serviceaccount.yaml` - Service account
- `02-engine-configmap.yaml` - Engine configuration
- `03-rivet-engine-deployment.yaml` - Main engine deployment
- `04-rivet-engine-service.yaml` - Service definition
- `05-rivet-engine-hpa.yaml` - Horizontal Pod Autoscaler
- `06-rivet-engine-singleton-deployment.yaml` - Singleton services
- `07-nats-configmap.yaml` - NATS configuration
- `08-nats-statefulset.yaml` - NATS cluster
- `09-nats-service.yaml` - NATS service
- `10-postgres-configmap.yaml` - PostgreSQL configuration
- `11-postgres-secret.yaml` - PostgreSQL credentials
- `12-postgres-statefulset.yaml` - PostgreSQL database
- `13-postgres-service.yaml` - PostgreSQL service

## Local Development

For local development with k3d:

```bash
./scripts/run/k8s/engine.sh
```

This script creates a k3d cluster, builds the image, and deploys everything.

## Production Deployment

For production deployments, see the steps outlined in our [Kubernetes Self-Hosting Guide](https://rivet.gg/docs/self-hosting/kubernetes).
