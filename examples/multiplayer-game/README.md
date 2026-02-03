# Multiplayer Game

A real-time Agar.io style arena showing a matchmaker coordinator and GameRoom data actors.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/multiplayer-game
pnpm install
pnpm dev
```

## Features

- **Coordinator pattern**: Matchmaker Rivet Actor that indexes and assigns GameRoom actors
- **Real-time events**: Player joins, movement, and collisions broadcast to connected clients
- **Stateful gameplay**: Persistent room state with player growth and collision resolution
- **Typed React client**: `@rivetkit/react` hooks for actions and event subscriptions

## Implementation

- **Matchmaker and GameRoom actors** ([`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/multiplayer-game/src/actors.ts)): Coordinator actor for room discovery plus GameRoom state and physics
- **Server routing** ([`src/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/multiplayer-game/src/server.ts)): Hono server exposing the Rivet Actor handler
- **Canvas frontend** ([`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/multiplayer-game/frontend/App.tsx)): Canvas rendering, input handling, and leaderboard UI

## Resources

Read more about [design patterns](/docs/actors/design-patterns), [actions](/docs/actors/actions), [events](/docs/actors/events), and [state](/docs/actors/state).

## License

MIT
