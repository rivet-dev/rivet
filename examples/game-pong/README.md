# Pong - Game Physics Example

A real-time multiplayer Pong game demonstrating how to use Rivet Actors for matchmaking and server-authoritative game state management.

## Getting Started

```bash
# Install dependencies
pnpm install

# Start the development server
pnpm dev
```

This will start both the backend actor server and the frontend Vite dev server. Open two browser windows to test multiplayer.

## Features

- **Matchmaking system**: A dedicated matchmaker actor pairs players together before starting games
- **Server-authoritative game loop**: Game physics run on the actor using `setInterval` in `onWake`
- **Player assignment**: Players are automatically assigned to left/right paddles via `onConnect`
- **Real-time state synchronization**: Game state broadcasts to all connected clients at 60 FPS
- **Spectator support**: Additional connections beyond 2 players watch as spectators

## Implementation

This example demonstrates two actor patterns working together:

**Matchmaker Actor** - Coordinates player matchmaking:
- Maintains a queue of waiting players
- Pairs players and creates unique match IDs
- Tracks active matches

**Pong Game Actor** - Handles actual gameplay:
- Uses `onWake` to start a `setInterval` game loop
- Uses `onSleep` to clean up the interval
- Uses `onConnect` to assign players to paddles (first = left, second = right)
- Uses `onDisconnect` to handle player departures
- Broadcasts game state updates to all connected clients

The game update function receives an `ActorContextOf<typeof pongGame>` to access state and broadcast events.

See the implementation in [`src/backend/registry.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/game-physics/src/backend/registry.ts).

## Resources

Read more about [lifecycle hooks](/docs/actors/lifecycle), [connection events](/docs/actors/connections), and [helper types](/docs/actors/helper-types).

## License

MIT
