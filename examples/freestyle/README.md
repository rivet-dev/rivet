# Freestyle Deployment for Rivet

Example project demonstrating serverless deployment of Rivet actors to [Freestyle](https://freestyle.sh) with [Rivet](https://www.rivet.dev/).

[Learn More →](https://github.com/rivet-dev/rivet)

[Discord](https://rivet.dev/discord) — [Documentation](https://www.rivet.dev/) — [Issues](https://github.com/rivet-dev/rivet/issues)


## What is this?

Freestyle is unique from other providers since it is built to deploy untrusted AI-generated or user-generated code. This enables your application to deploy vibe-coded or user-provided backends on Rivet and Freestyle. This example showcases a real-time stateful chat app that can be deployed to FreeStyle's [Web Deployment](https://docs.freestyle.sh/web/overview) platform.

## Getting Started

### Prerequisites

- Node.js 18+
- Deno (for development)

**Note**: Deno is required since Freestyle uses Deno for their Web Deployments under the hood

### Installation

```sh
git clone https://github.com/rivet-dev/rivet
cd rivet/examples/freestyle
pnpm install
```

### Development

```sh
pnpm run dev
```

Open your browser to `http://localhost:5173` to see the application.

```sh
RIVET_RUNNER_KIND=serverless VITE_RIVET_ENDPOINT="$RIVET_ENDPOINT" pnpm run dev
```

### Deploy to Freestyle

```sh
# Set env vars
export FREESTYLE_DOMAIN="my-domain.style.dev"   # Set this to any unique *.style.dev domain
export FREESTYLE_API_KEY="XXXX"                 # See https://admin.freestyle.sh/dashboard/api-tokens
export RIVET_ENDPOINT="http://api.rivet.gg"
export RIVET_NAMESPACE="XXXX"                   # Creates new namespace if does not exist
export RIVET_TOKEN="XXXX"                       # Rivet Service token
export RIVET_PUBLISHABLE_TOKEN="XXXX"           # For connecting to Rivet Actors

pnpm run freestyle:deploy
```

Open your browser to your Freestyle domain to see your application connect to Rivet deployed on Freestyle.

If self-hosting Rivet:
1. **Important**: `RIVET_ENDPOINT` must be public to the internet.
2. `RIVET_PUBLISHABLE_TOKEN` can be kept empty.

## License

Apache 2.0
