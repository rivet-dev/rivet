# TanStack Start

Example demonstrating RivetKit integration with TanStack Start, featuring a real-time counter with persistent state.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/tanstack-start
pnpm install
pnpm dev
```

## Features

- **TanStack Start integration**: Seamless RivetKit setup using TanStack's file-based routing and server handlers
- **Persistent actor state**: Counter state managed by Rivet Actors that survives restarts
- **Real-time synchronization**: Counter values broadcast to all connected clients via events
- **React hooks**: Uses `@rivetkit/react` with `useActor` for type-safe actor connections

## Implementation

This example shows how to integrate RivetKit with TanStack Start using file-based routing:

- **Actor Definition** ([`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/tanstack-start/src/actors.ts)): Counter actor with persistent state, increment action, and broadcast events
- **Route Handler** ([`src/routes/api.rivet.$.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/tanstack-start/src/routes/api.rivet.$.tsx)): TanStack Start route that handles RivetKit requests
- **React Component** ([`src/components/Counter.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/tanstack-start/src/components/Counter.tsx)): Counter UI using `useActor` hook and event subscriptions

## Resources

Read more about [actions](/docs/actors/actions), [state](/docs/actors/state), and [events](/docs/actors/events).

## License

MIT
