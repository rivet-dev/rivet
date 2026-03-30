# Svelte Counter

A real-time counter built with Svelte 5 and RivetKit. Multiple browser tabs stay in sync automatically via actor state and broadcast events.

## Getting Started

```bash
# Install dependencies
pnpm install

# Start the development server (backend + frontend)
pnpm dev
```

Open [http://localhost:5173](http://localhost:5173) in multiple tabs to see the counter sync in real time.

## Features

- **Persistent actor state** - The counter value survives server restarts and reconnections
- **Real-time sync** - All connected clients update instantly via `broadcast` events
- **Svelte 5 runes** - Reactive actor state integrates directly with `$state` and `$effect`
- **Named instances** - Change the counter name in the input to connect to a different actor instance
- **Type-safe client** - Full TypeScript types flow from the actor definition to the Svelte component

## Implementation

The backend defines a `counter` actor with persistent state and a `newCount` broadcast event:

See [`src/index.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/svelte/src/index.ts).

The frontend creates a single shared RivetKit instance and uses `useActor()` inside the Svelte component to connect and subscribe to events:

See [`frontend/rivet.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/svelte/frontend/rivet.ts) and [`frontend/App.svelte`](https://github.com/rivet-dev/rivet/tree/main/examples/svelte/frontend/App.svelte).

`useActor()` accepts a getter function, so changing `counterName` automatically reconnects to the new actor instance. `onEvent()` subscribes to broadcast events and updates local `$state`.

## Resources

- [Actors overview](https://rivet.dev/docs/actors)
- [Actor state](https://rivet.dev/docs/actors/state)
- [Events and broadcast](https://rivet.dev/docs/actors/events)

## License

MIT
