# Matchmaking And Session Patterns

Example project demonstrating seven multiplayer game Rivet Actor scaffolds:

- io-style (open lobby, 10 tps)
- competitive (filled room, mode + team assignment, 20 tps)
- party (host start + party code, no tick loop)
- async turn-based (invite + open pool, no tick loop)
- open world (chunk index + chunk actors, 10 tps chunk loop)
- ranked (ELO queueing, 20 tps)
- battle royale (queue threshold start, 10 tps)

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
- `src/actors/open-world/world-index.ts`: Coordinator that resolves world positions into chunk actor keys
- `src/actors/open-world/chunk.ts`: Chunk actor keyed by `[worldId, chunkX, chunkY]` for sharded world state
- `frontend/App.tsx`: Simple React UI that runs scripted matchmaking demos
- `tests/matchmaking-and-session-patterns.test.ts`: Golden-path unit tests with simulated multi-player flows

## License

MIT
