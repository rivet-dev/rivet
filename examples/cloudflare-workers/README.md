# Cloudflare Workers

Example project demonstrating Cloudflare Workers deployment.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/cloudflare-workers
npm install
npm run dev
```


## Features

- **Cloudflare Workers integration**: Deploy Rivet Actors to Cloudflare's edge network using Durable Objects
- **Edge-native execution**: Actors run at the edge for low-latency global access
- **Native Durable Object SQLite**: Actor state is persisted through Cloudflare's built-in Durable Object SQLite storage
- **Built-in HTTP API**: Actors automatically exposed via HTTP endpoints
- **Wrangler CLI integration**: Standard Cloudflare tooling for development and deployment

## Implementation

This example demonstrates deploying Rivet Actors to Cloudflare Workers with native Durable Object SQLite:

- **Actor Definition** ([`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/cloudflare-workers/src/actors.ts)): Shows how to set up a SQLite-backed actor for Cloudflare Workers using Durable Objects

## Resources

Read more about [Cloudflare Workers integration](/docs/platforms/cloudflare-workers), [actions](/docs/actors/actions), and [state](/docs/actors/state).

## License

MIT
