# Agario Clone - Game Example

A real-time multiplayer Agario-style game demonstrating how to use Rivet Actors for server-authoritative game state with many concurrent players.

## Getting Started

```bash
# Install dependencies
pnpm install

# Start the development server
pnpm dev
```

This will start both the backend actor server and the frontend Vite dev server. Open multiple browser windows to test multiplayer.

## Features

- **Server-authoritative game loop**: Game physics run on the actor using `setInterval` in `onWake`
- **Player management**: Players join/leave via `onConnect`/`onDisconnect`
- **Real-time state synchronization**: Game state broadcasts to all connected clients at 60 FPS
- **Collision detection**: Players can eat smaller players to grow

## Implementation

This example demonstrates a single game room actor:

**Game Actor** - Handles all gameplay:
- Uses `onWake` to start a `setInterval` game loop
- Uses `onSleep` to clean up the interval
- Uses `onConnect` to spawn new players
- Uses `onDisconnect` to remove players
- Broadcasts game state updates to all connected clients

The game update function receives an `ActorContextOf<typeof gameRoom>` to access state and broadcast events.

See the implementation in [`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/game-agario/src/backend/registry.ts).

## Resources

Read more about [lifecycle hooks](/docs/actors/lifecycle), [connection events](/docs/actors/connections), and [helper types](/docs/actors/helper-types).

## License

MIT
