# Hello World - Cloudflare Workers

A minimal Rivet Actor counter running on Cloudflare Workers with the WebAssembly runtime.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/hello-world-cloudflare-workers
npm install
npm run dev
```

`rivet dev` runs a local Rivet engine and spawns `wrangler dev` for you.

## Implementation

The Worker exports `createHandler` from `@rivetkit/cloudflare-workers`, which wires the WebAssembly runtime and serves the Rivet handler. `RIVET_ENDPOINT` is the only required variable, set in [`wrangler.toml`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world-cloudflare-workers/wrangler.toml).

- Worker entry ([`src/index.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world-cloudflare-workers/src/index.ts)): Counter actor and `createHandler`.

## Resources

Read more about [actions](/docs/actors/actions) and [state](/docs/actors/state), or follow the [Cloudflare Workers Quickstart](/docs/actors/quickstart/cloudflare) and [deploy guide](/docs/deploy/cloudflare).

## License

MIT
