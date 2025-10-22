# Cloudflare Workers for RivetKit

Example project demonstrating Cloudflare Workers deployment with [RivetKit](https://rivetkit.org).

[Learn More →](https://github.com/rivet-dev/rivetkit)

[Discord](https://rivet.dev/discord) — [Documentation](https://rivetkit.org) — [Issues](https://github.com/rivet-dev/rivetkit/issues)

## Getting Started

### Prerequisites

- Node.js
- Cloudflare account with Actors enabled
- Wrangler CLI installed globally (`npm install -g wrangler`)

### Installation

```sh
git clone https://github.com/rivet-dev/rivetkit
cd rivetkit/examples/cloudflare-workers
npm install
```

### Development

```sh
npm run dev
```

This will start the Cloudflare Workers development server locally at http://localhost:8787.

### Testing the Client

In a separate terminal, run the client script to interact with your actors:

```sh
npm run client
```

### Deploy to Cloudflare

First, authenticate with Cloudflare:

```sh
wrangler login
```

Then deploy:

```sh
npm run deploy
```

## License

Apache 2.0
