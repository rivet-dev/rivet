# Cloudflare Workers with Hono

Example project demonstrating Cloudflare Workers deployment with Hono router.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/cloudflare-workers-hono
npm install
npm run dev
```


## Features

- **Cloudflare Workers integration**: Deploy Rivet Actors to Cloudflare's edge network using Durable Objects
- **Hono routing**: Use Hono web framework for HTTP request handling
- **Edge-native execution**: Actors run at the edge for low-latency global access
- **Type-safe API endpoints**: Full TypeScript support across actor and HTTP layers

## Implementation

This example demonstrates combining Hono with Rivet Actors on Cloudflare Workers:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/cloudflare-workers-hono/src/backend/registry.ts)): Shows how to integrate Hono router with actors on Cloudflare Workers

## Resources

Read more about [Cloudflare Workers integration](/docs/platforms/cloudflare-workers) and [actions](/docs/actors/actions).

## License

MIT
