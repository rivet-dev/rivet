> **Note:** This is the Render-optimized version of the [actor-actions](../actor-actions) example.
> It adds a production HTTP server, Vite build, and a [`render.yaml`](./render.yaml) Blueprint for deploying to Render.

[![Deploy to Render](https://render.com/images/deploy-to-render-button.svg)](https://render.com/deploy?repo=https://github.com/rivet-dev/rivet/tree/main/examples/actor-actions-render)

# Actor Actions

Demonstrates how to define and call actions on Rivet Actors for RPC-style communication between actors and clients.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/actor-actions
npm install
npm run dev
```


## Features

- **Type-safe actor actions**: Define and call actions with full TypeScript type safety
- **Actor state management**: Initialize and persist actor state using `createState`
- **Cross-actor communication**: Create and interact with actors from within other actors
- **RPC-style patterns**: Call actor methods from client code with automatic type inference

## Implementation

This example demonstrates the fundamentals of defining and calling actor actions:

- **Actor Definition** ([`src/actors.ts`](./src/actors.ts)): `company` creates and tracks `employee` actors via cross-actor actions
- **Server Setup** ([`src/server.ts`](./src/server.ts)): Hono app routing `/api/rivet/*` to the registry handler
- **React Frontend** ([`frontend/app/App.tsx`](./frontend/app/App.tsx)): Imperative `createClient` with `ActorError` handling

## Deploy on Render

1. Set **Root Directory** to `examples/actor-actions-render` if deploying from the monorepo.
2. Add the following environment variables in your Render service:

| Variable | Description |
|----------|-------------|
| `RIVET_ENDPOINT` | Backend endpoint URL from your [Rivet Cloud](https://hub.rivet.dev) project |
| `RIVET_PUBLIC_ENDPOINT` | Public endpoint URL from your [Rivet Cloud](https://hub.rivet.dev) project |

3. In the Rivet dashboard, point **Connect your backend** at your service's HTTPS URL.

> **`RIVET_ENVOY_VERSION`** is automatically derived from Render's `RENDER_GIT_COMMIT` — no manual bump needed per deploy. Set it explicitly to override.

## Resources

- [RivetKit documentation](https://rivet.dev/docs)
- [Self-host Rivet Engine on Render](https://rivet.dev/docs/self-hosting/render)
- [Render Blueprint specification](https://render.com/docs/blueprint-spec)

## License

MIT
