> **Note:** This is the Render-optimized version of the [hello-world](https://github.com/rivet-dev/rivet/tree/main/examples/hello-world) example.
> It adds a production HTTP server, Vite build, and a [`render.yaml`](./render.yaml) Blueprint for deploying to Render.

[![Deploy to Render](https://render.com/images/deploy-to-render-button.svg)](https://render.com/deploy?repo=https://github.com/rivet-dev/rivet/tree/main/examples/hello-world-render)

# Hello World

A minimal example demonstrating RivetKit with a real-time counter shared across multiple clients, deployed on [Render](https://render.com/) with [Rivet Cloud](https://rivet.dev/).

## Features

- **Actor state management**: Persistent counter state managed by Rivet Actors
- **Real-time updates**: Counter values synchronized across all connected clients via events
- **Multiple actor instances**: Each counter ID creates a separate actor instance
- **React integration**: Uses `@rivetkit/react` for seamless React hooks integration

## Implementation

This example demonstrates the core RivetKit concepts with a simple counter:

- **Actor Definition** ([`src/rivet/counter.ts`](./src/rivet/counter.ts)): Counter actor with persistent state and broadcast events
- **Server Setup** ([`src/http/server.ts`](./src/http/server.ts)): Production HTTP server that serves the built client and forwards `/api/rivet/*` to the registry handler
- **React Frontend** ([`frontend/app/App.tsx`](./frontend/app/App.tsx)): Counter component using `useActor` hook and event subscriptions

## Deploy on Render

1. Set **Root Directory** to `examples/hello-world-render` if deploying from the monorepo.
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
