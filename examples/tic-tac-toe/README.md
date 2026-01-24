# Tic-Tac-Toe

A minimal multiplayer tic-tac-toe game demonstrating real-time game state synchronization with Rivet Actors.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/tic-tac-toe
npm install
npm run dev
```

## Features

- **Real-time multiplayer**: Two players see moves instantly via WebSocket events
- **Persistent game state**: Board state survives actor restarts
- **Turn-based logic**: Only the current player can make moves
- **Win/draw detection**: Automatic detection of game end conditions
- **Spectator mode**: Additional users can watch as viewers when lobby is full
- **Multiple lobbies**: Each lobby ID creates a separate actor instance

## Implementation

This example demonstrates core Rivet Actor patterns for real-time game synchronization:

- **Actor Definition** ([`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/tic-tac-toe/src/actors.ts)): Defines the `ticTacToe` actor with game state (board, players, winner) and actions for joining, making moves, and resetting

Key patterns used:
- `c.state` for persistent game state that survives restarts
- `c.broadcast("gameUpdate", state)` to push updates to all connected clients
- `game.useEvent("gameUpdate", callback)` to subscribe to real-time updates

## Resources

Read more about [state](/docs/actors/state), [actions](/docs/actors/actions), and [events](/docs/actors/events).

## License

MIT
