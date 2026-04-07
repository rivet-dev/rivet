> **Note:** This is the Render-optimized version of the [stream](../stream) example.
> It adds a production HTTP server, Vite build, and a [`render.yaml`](./render.yaml) Blueprint for deploying to Render.

[![Deploy to Render](https://render.com/images/deploy-to-render-button.svg)](https://render.com/deploy?repo=https://github.com/rivet-dev/rivet/tree/main/examples/stream-render)

# Stream Processor

Example project demonstrating real-time top-K stream processing.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/stream
npm install
npm run dev
```


## Features

- **Top-K Processing**: Maintains the top 3 highest values in real-time
- **Real-time Updates**: All connected clients see changes immediately
- **Stream Statistics**: Total count, highest value, and live metrics
- **Interactive Input**: Add custom values or generate random numbers
- **Reset Functionality**: Clear the stream and start fresh
- **Responsive Design**: Clean, modern interface with live statistics

## Implementation

This stream processor uses a Top-K algorithm to efficiently maintain the top 3 values using insertion sort. Updates are instantly sent to all connected clients via event broadcasting.

- **Actor Definition** ([`src/actors.ts`](./src/actors.ts)): `streamProcessor` maintaining a sorted top-3 leaderboard with broadcast updates
- **Server Setup** ([`src/server.ts`](./src/server.ts)): Hono app routing `/api/rivet/*` to the registry handler
- **React Frontend** ([`frontend/app/App.tsx`](./frontend/app/App.tsx)): `useActor` hook with live stats and event-driven UI

## Deploy on Render

1. Set **Root Directory** to `examples/stream-render` if deploying from the monorepo.
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
