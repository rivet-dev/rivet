> **Note:** This is the Render-optimized version of the [state](../state) example.
> It adds a production HTTP server, Vite build, and a [`render.yaml`](./render.yaml) Blueprint for deploying to Render.

[![Deploy to Render](https://render.com/images/deploy-to-render-button.svg)](https://render.com/deploy?repo=https://github.com/rivet-dev/rivet)

# State Management

Demonstrates persistent state management in Rivet Actors with automatic state saving and restoration.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/state
npm install
npm run dev
```


## Features

- **Persistent state**: Actor state automatically saved and restored across restarts
- **Typed state management**: Full TypeScript type safety for state objects
- **State initialization**: Define initial state with `createState` or `state` property
- **Automatic serialization**: State changes automatically persisted without manual saves

## Implementation

This example demonstrates state management in Rivet Actors:

- **Actor Definition** ([`src/actors.ts`](./src/actors.ts)): `chatRoom` with persistent message list, `sendMessage`, `clearMessages`, and broadcast events
- **Server Setup** ([`src/server.ts`](./src/server.ts)): Hono app routing `/api/rivet/*` to the registry handler
- **React Frontend** ([`frontend/app/App.tsx`](./frontend/app/App.tsx)): `useActor` hook with real-time event subscriptions and auto-scroll

## Deploy on Render

1. Set **Root Directory** to `examples/state-render` if deploying from the monorepo.
2. Add the following environment variables in your Render service:

| Variable | Description |
|----------|-------------|
| `RIVET_ENDPOINT` | Server credential URL (`sk_…`) from [dashboard.rivet.dev](https://dashboard.rivet.dev) |
| `RIVET_PUBLIC_ENDPOINT` | Client credential URL (`pk_…`), embedded at build time |

3. In the Rivet dashboard, point **Connect your backend** at your service's HTTPS URL.

> **`RIVET_ENVOY_VERSION`** is automatically derived from Render's `RENDER_GIT_COMMIT` — no manual bump needed per deploy. Set it explicitly to override.

## Resources

- [RivetKit documentation](https://rivet.dev/docs)
- [Self-host Rivet Engine on Render](https://rivet.dev/docs/self-hosting/render)
- [Render Blueprint specification](https://render.com/docs/blueprint-spec)

## License

MIT
