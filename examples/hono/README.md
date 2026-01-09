# Hono Integration

Example project demonstrating Hono web framework integration.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/hono
npm install
npm run dev
```


## Features

- **Hono web framework**: Lightweight, fast HTTP routing for actor APIs
- **Actor integration**: Call actor actions from Hono route handlers
- **Type-safe endpoints**: Full TypeScript type safety across HTTP and actor layers
- **Middleware support**: Use Hono middleware for authentication, logging, and more

## Implementation

This example demonstrates integrating Hono web framework with Rivet Actors:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/hono/src/backend/registry.ts)): Shows how to use Hono for HTTP routing with actors

## Deployment

This example can be deployed to Railway. See the [Railway deployment guide](/docs/connect/railway) for detailed instructions.

[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/new/template?template=https://github.com/rivet-dev/rivet/tree/main/examples/hono)

### Required Environment Variables

Configure these environment variables in your Railway project:
- `RIVET_ENDPOINT` - Your Rivet Engine endpoint
- `RIVET_NAMESPACE` - Your Rivet namespace
- `RIVET_RUNNER_TOKEN` - Your Rivet runner token

Get these values from the [Rivet dashboard](https://dashboard.rivet.dev) under Connect > Railway.

## Resources

Read more about [actions](/docs/actors/actions) and [setup](/docs/setup).

## License

MIT
