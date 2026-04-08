> **Note:** This is the Render-optimized version of the [chat-room](../chat-room) example.
> It adds a production HTTP server, Vite build, and a [`render.yaml`](./render.yaml) Blueprint for deploying to Render.

[![Deploy to Render](https://render.com/images/deploy-to-render-button.svg)](https://render.com/deploy?repo=https://github.com/rivet-dev/rivet/tree/main/examples/chat-room-render)

# Chat Room

Example project demonstrating real-time messaging and actor state management.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/chat-room
npm install
npm run dev
```


## Features

- **Real-time messaging**: Broadcast messages to all connected clients instantly
- **Persistent chat history**: Messages automatically saved in actor state across restarts
- **Multiple chat rooms**: Each room is a separate actor instance with isolated state
- **Event-driven architecture**: Use actor events to push updates to clients in real-time

## Implementation

The chat room demonstrates core Rivet Actor patterns for real-time communication:

- **Actor Definition** ([`src/actors.ts`](./src/actors.ts)): `chatRoom` actor with persistent message history and `newMessage` broadcast events
- **Server Setup** ([`src/server.ts`](./src/server.ts)): Hono app routing `/api/rivet/*` to the registry handler
- **React Frontend** ([`frontend/app/App.tsx`](./frontend/app/App.tsx)): `useActor` hook with real-time event subscriptions

## Deploy on Render

1. Set **Root Directory** to `examples/chat-room-render` if deploying from the monorepo.
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
