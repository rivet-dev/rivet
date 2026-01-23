> **Note:** This is the Vercel-optimized version of the [elysia](../elysia) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Felysia-vercel&project-name=elysia-vercel)

# Elysia Integration

Example project demonstrating Elysia web framework integration.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/elysia
npm install
npm run dev
```


## Features

- **Elysia web framework**: Use Elysia for HTTP routing and request handling with actors
- **High-performance routing**: Built on Bun for fast request processing
- **Type-safe endpoints**: Full TypeScript type safety across HTTP and actor layers
- **Actor integration**: Call actor actions from Elysia route handlers

## Implementation

This example demonstrates integrating Elysia web framework with Rivet Actors:

- **Actor Definition** ([`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/elysia/src/backend/registry.ts)): Shows how to use Elysia router with actors for HTTP endpoints

## Resources

Read more about [actions](/docs/actors/actions) and [setup](/docs/setup).

## License

MIT
