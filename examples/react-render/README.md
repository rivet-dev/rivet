> **Note:** This is the Render-optimized version of the [react](../react) example.
> It adds a production HTTP server, Vite build, and a [`render.yaml`](./render.yaml) Blueprint for deploying to Render.

[![Deploy to Render](https://render.com/images/deploy-to-render-button.svg)](https://render.com/deploy?repo=https://github.com/rivet-dev/rivet/tree/main/examples/react-render)

# React Integration

Demonstrates React frontend integration with Rivet Actors.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/react
npm install
npm run dev
```


## Features

- **React frontend**: Build interactive UIs with React that connect to Rivet Actors
- **Type-safe client**: Use `@rivetkit/react` hooks for type-safe actor communication
- **Real-time updates**: Subscribe to actor events for live UI updates
- **Actor state management**: Actors handle backend logic while React manages UI state

## Implementation

This example demonstrates React frontend integration with Rivet Actors:

- **Actor Definition** ([`src/actors.ts`](./src/actors.ts)): Minimal counter with `increment` action and `newCount` event
- **Server Setup** ([`src/server.ts`](./src/server.ts)): Hono app routing `/api/rivet/*` to the registry handler
- **React Frontend** ([`frontend/app/App.tsx`](./frontend/app/App.tsx)): `useActor` hook with event subscription

## Deploy on Render

1. Set **Root Directory** to `examples/react-render` if deploying from the monorepo.
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
