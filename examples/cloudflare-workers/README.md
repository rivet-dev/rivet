# Cloudflare Workers for Rivet

Example project demonstrating Cloudflare Workers deployment with [Rivet](https://www.rivet.dev/).

[Learn More →](https://github.com/rivet-dev/rivet)

[Discord](https://rivet.dev/discord) — [Documentation](https://www.rivet.dev/) — [Issues](https://github.com/rivet-dev/rivet/issues)

## Getting Started

### Prerequisites

- Node.js
- Cloudflare account with Actors enabled
- Wrangler CLI installed globally (`npm install -g wrangler`)

### Installation

```sh
git clone https://github.com/rivet-dev/rivet
cd rivet/examples/cloudflare-workers
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
