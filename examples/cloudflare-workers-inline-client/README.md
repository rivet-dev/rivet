# Cloudflare Workers Inline Client Example

Simple example demonstrating accessing Rivet Actors via Cloudflare Workers without exposing a public API. This uses the `createInlineClient` function to connect directly to your Durable Object.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/cloudflare-workers-inline-client
npm install
npm run dev
```


## Features

- **Inline client access**: Call actor actions directly from Cloudflare Worker without HTTP overhead
- **Private actor APIs**: Actors not exposed via public HTTP endpoints
- **Edge-native execution**: Actors and workers run together on Cloudflare's edge network
- **Type-safe communication**: Full TypeScript type safety between worker and actor

## Implementation

This example demonstrates using inline clients to call actors privately within Cloudflare Workers:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/cloudflare-workers-inline-client/src/backend/registry.ts)): Shows how to use `createInlineClient` for direct actor access without public HTTP endpoints

## Resources

Read more about [Cloudflare Workers integration](/docs/platforms/cloudflare-workers) and [actions](/docs/actors/actions).

## License

MIT
