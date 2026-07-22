# Vercel Workflows (Beta)

A minimal Vercel Workflows app backed by Rivet Actors.

This integration is in beta.

## Prerequisites

- Node.js 22 or newer
- pnpm

## Getting Started

```sh
cp .env.example .env
pnpm build
pnpm dev
```

Start an order with `curl -X POST http://localhost:3000/orders/42`. The first
World operation lazily starts Rivet in the same process and waits until it is
ready; there is no separate Rivet server command.

Run `pnpm test` for Vercel's official `@workflow/vitest` integration test.
The World package itself runs Vercel's `@workflow/world-testing` suite against
an owned Rivet engine.

## Features

- Vercel World backed by Rivet Actors
- Durable steps and waits
- Vercel's supported Vitest workflow harness

## Implementation

- [Workflow](https://github.com/rivet-dev/rivet/blob/main/examples/vercel-workflow/workflows/order.ts)
- [Integration test](https://github.com/rivet-dev/rivet/blob/main/examples/vercel-workflow/workflows/order.integration.test.ts)
- [HTTP server](https://github.com/rivet-dev/rivet/blob/main/examples/vercel-workflow/src/server.ts)

## Resources

- [Rivet Vercel Workflows guide](https://rivet.dev/integrations/vercel-workflow)
- [Vercel Workflows](https://useworkflow.dev)

## License

MIT
