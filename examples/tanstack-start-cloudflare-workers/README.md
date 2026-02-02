# TanStack Start on Cloudflare Workers

Example demonstrating RivetKit integration with TanStack Start deployed on Cloudflare Workers, featuring a real-time counter with persistent state.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/tanstack-start-cloudflare-workers
pnpm install
pnpm dev
```

## Features

- **Cloudflare Workers deployment**: TanStack Start app running on Cloudflare Workers with RivetKit
- **Persistent actor state**: Counter state managed by Rivet Actors that survives restarts
- **Real-time synchronization**: Counter values broadcast to all connected clients via events
- **React hooks**: Uses `@rivetkit/react` with `useActor` for type-safe actor connections

## Implementation

This example shows how to integrate RivetKit with TanStack Start on Cloudflare Workers:

- **Actor Definition** ([`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/tanstack-start-cloudflare-workers/src/actors.ts)): Counter actor with persistent state, increment action, and broadcast events
- **Server Entry** ([`src/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/tanstack-start-cloudflare-workers/src/server.ts)): Custom server entry that routes RivetKit requests through the Cloudflare Workers handler
- **React Component** ([`src/components/Counter.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/tanstack-start-cloudflare-workers/src/components/Counter.tsx)): Counter UI using `useActor` hook and event subscriptions

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT
