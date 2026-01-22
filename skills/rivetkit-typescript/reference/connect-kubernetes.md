# Deploying to Kubernetes

> Source: `src/content/docs/connect/kubernetes.mdx`
> Canonical URL: https://rivet.gg/docs/connect/kubernetes
> Description: Deploy your RivetKit app to any Kubernetes cluster.

---
## Steps

### Prerequisites

- A Kubernetes cluster with `kubectl` access (EKS, GKE, k3s, etc.)
- Container registry credentials (Docker Hub, GHCR, GCR, etc.)
- Your RivetKit app
  - If you don't have one, see the [Quickstart](/docs/actors/quickstart) page or our [Examples](https://github.com/rivet-dev/rivet/tree/main/examples)
- Access to the [Rivet Cloud](https://dashboard.rivet.dev/) or a [self-hosted Rivet Engine](/docs/general/self-hosting)

### Package Your App

Create a `Dockerfile` in your project root:

```dockerfile
FROM node:20-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --omit=dev
COPY . .
ENV PORT=8080
CMD ["node", "server.js"]
```

### Build and Push the Image

```bash
docker build -t registry.example.com/your-team/rivetkit-app:latest .
docker push registry.example.com/your-team/rivetkit-app:latest
```

Replace `registry.example.com/your-team` with your registry path. Auth with `docker login` first if needed.

### Set Environment Variables

After creating your project on the Rivet dashboard, select Kubernetes as your provider. You'll be provided `RIVET_ENDPOINT` and `RIVET_PUBLIC_ENDPOINT` environment variables.

Create a `rivetkit-secrets.yaml` for your environment variables:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: rivetkit-secrets
type: Opaque
stringData:
  RIVET_ENDPOINT: <your-rivet-endpoint>
  RIVET_PUBLIC_ENDPOINT: <your-rivet-public-endpoint>
```

### Deploy to Kubernetes

Create a `deployment.yaml`:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rivetkit-app
spec:
  replicas: 1
  selector:
    matchLabels:
      app: rivetkit-app
  template:
    metadata:
      labels:
        app: rivetkit-app
    spec:
      containers:
        - name: rivetkit-app
          image: registry.example.com/your-team/rivetkit-app:latest
          envFrom:
            - secretRef:
                name: rivetkit-secrets
```

Apply both manifests:

```bash
kubectl apply -f rivetkit-secrets.yaml
kubectl apply -f deployment.yaml
```

### Connect to Rivet

1. Add a `Service` and Ingress to expose your app externally (e.g. `my-app.example.com`)
2. On the Rivet dashboard, paste your domain with the `/api/rivet` path into the connect form (e.g. `https://my-app.example.com/api/rivet`)
3. Click "Done"

### Verify

Check that the pod is running:

```bash
kubectl get pods -l app=rivetkit-app
```

Your app should appear as connected on the Rivet dashboard once the pod is ready.

_Source doc path: /docs/connect/kubernetes_
