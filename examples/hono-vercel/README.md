> **Note:** This is the Vercel-optimized version of the [hono](../hono) example.
> It uses the `hono/vercel` adapter and is configured for Vercel deployment.

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https%3A%2F%2Fgithub.com%2Frivet-gg%2Frivet%2Ftree%2Fmain%2Fexamples%2Fhono-vercel&project-name=hono-vercel)

# Hono Integration

Build type-safe HTTP APIs with Hono web framework and RivetKit Actors. Features lightweight routing, middleware support, and seamless actor integration.

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

## Resources

Read more about [actions](/docs/actors/actions) and [setup](/docs/setup).

## License

MIT
