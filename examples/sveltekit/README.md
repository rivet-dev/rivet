# SvelteKit Counter

A real-time counter built with SvelteKit and RivetKit. Multiple browser tabs stay in sync automatically via actor state and broadcast events.

## Getting Started

```bash
# Install dependencies
pnpm install

# Start the development server (backend + SvelteKit)
pnpm dev
```

Open [http://localhost:5173](http://localhost:5173) in multiple tabs to see the counter sync in real time.

## Features

- **Persistent actor state** - The counter value survives server restarts and reconnections
- **Real-time sync** - All connected clients update instantly via `broadcast` events
- **Svelte context pattern** - `createRivetContext` provides a typed RivetKit instance to all routes via SvelteKit's layout system
- **Named instances** - Change the counter name in the input to connect to a different actor instance
- **Type-safe client** - Full TypeScript types flow from the actor definition through the layout to the page component

## Implementation

The backend actor runs as a separate process. It is defined in [`server/index.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/sveltekit/server/index.ts) and started alongside the SvelteKit dev server via `concurrently`.

A typed RivetKit context is created in [`src/lib/rivet.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/sveltekit/src/lib/rivet.ts) using `createRivetContext`. This follows the recommended pattern for SvelteKit apps - one context instance shared across routes.

The root layout at [`src/routes/+layout.svelte`](https://github.com/rivet-dev/rivet/tree/main/examples/sveltekit/src/routes/+layout.svelte) calls `rivetContext.setup()` to initialize the client and make it available to all child routes.

The counter page at [`src/routes/+page.svelte`](https://github.com/rivet-dev/rivet/tree/main/examples/sveltekit/src/routes/+page.svelte) retrieves the context with `rivetContext.get()`, calls `useActor()` to connect, and uses `onEvent()` to subscribe to broadcast events.

## Resources

- [Actors overview](https://rivet.dev/docs/actors)
- [Actor state](https://rivet.dev/docs/actors/state)
- [Events and broadcast](https://rivet.dev/docs/actors/events)

## License

MIT
