# Hello World - Cloudflare Workers + Hono

A Rivet Actor counter running on Cloudflare Workers with [Hono](https://hono.dev) mounted alongside the Rivet handler.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/hello-world-cloudflare-workers-hono
npm install
npm run dev
```

`rivet dev` runs a local Rivet engine and spawns `wrangler dev` for you.

## Implementation

`createHandler` from `@rivetkit/cloudflare-workers` keeps the Rivet manager API on `/api/rivet` and forwards every other route to your Hono app via `{ fetch: app.fetch }`. Hono routes call actors with a client from `rivetkit/client`. `RIVET_ENDPOINT` is the only required variable.

See [`src/index.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world-cloudflare-workers-hono/src/index.ts).

## Resources

Read more about [actions](/docs/actors/actions) and [state](/docs/actors/state), or follow the [Cloudflare Workers Quickstart](/docs/actors/quickstart/cloudflare) and [deploy guide](/docs/deploy/cloudflare).

## License

MIT
