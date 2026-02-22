# Matchmaking And Session Patterns

Example project demonstrating multiplayer game Rivet Actor scaffolds:

- io-style (open lobby, 10 tps)

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/multiplayer-game-patterns
pnpm install
pnpm dev
```

## What This Example Covers

- `src/actors/<game-type>/matchmaker.ts`: SQLite-backed matchmaking coordinator
- `src/actors/<game-type>/match.ts`: Match scaffold with actions/events/lifecycle and optional tick loop
- `frontend/App.tsx`: Simple React UI that runs scripted matchmaking demos
- `tests/matchmaking-and-session-patterns.test.ts`: Golden-path unit tests with simulated multi-player flows

## License

MIT
