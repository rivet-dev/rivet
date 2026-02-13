# Open Lobby IO-Style Multiplayer (2D)

## Summary

A drop-in/drop-out arena (Agar.io-inspired) built with a matchmaker coordinator actor that automatically creates and destroys game room actors.

- Frontend: React + canvas.
- Backend: RivetKit actors, realtime events.
- Assets: Kenney (downloaded at build time, gitignored).

## Goals

- Open lobby matchmaking: join the most-full room under capacity; create rooms on demand.
- Room lifecycle: rooms destroy themselves when empty.
- Simple, fun loop: move, eat pellets, grow; collide to consume smaller players.
- Keep it secure-by-default: no spoofing other players; basic movement sanity checks.

## Non-goals

- Skill-based matchmaking.
- Advanced netcode (interpolation, prediction rollback).
- Strong anti-cheat beyond caps/rate limits.

## UX

- Landing: enter a display name, click Play.
- Game: top-down arena, minimap optional.
- HUD: your mass, leaderboard (top 10), player count.

## Actors

### `ioMatchmaker` (coordinator)

- Key: `["main"]`.
- DB: `rivetkit/db` raw SQLite for room indexing state that may grow large.
- State:
  - `rooms: Array<{ roomId: string; players: number; updatedAt: number; }>`
- SQLite schema:
  - `rooms(room_id TEXT PRIMARY KEY, players INTEGER NOT NULL, updated_at INTEGER NOT NULL)`
- Responsibilities:
  - Choose or create a room for a connecting player.
  - Track approximate room population to route players.
- Lifecycle:
  - `onMigrate`: create `rooms` table.
  - `onWake`: schedule periodic cleanup of stale rooms (e.g. every 5 minutes).
- Actions:
  - `findRoom(): { roomId: string }`
    - Select the most-full room where `players < ROOM_MAX`.
      - Prefer a SQLite query like `SELECT room_id FROM rooms WHERE players < ? ORDER BY players DESC, updated_at DESC LIMIT 1`.
    - If none exists, create a new `ioRoom` with a random `roomId` and register it.
  - `roomHeartbeat(roomId: string, players: number)`
    - Called by room periodically or on membership changes.
    - Upsert `rooms` row with `players` and `updated_at = Date.now()`.
  - `roomClosed(roomId: string)`
    - Called by the room right before it destroys.
    - Delete from `rooms`.
  - `cleanupStaleRooms()`
    - Delete rows where `updated_at < now - STALE_TTL_MS` to handle missed `roomClosed` calls.
    - This is defensive; rooms should call `roomClosed` on shutdown, but cleanup handles crashes/network failures.
- Events:
  - Optional for UI: `roomsUpdated(rooms)`.

### `ioRoom` (data)

- Key: `[roomId]`.
- State:
  - `playersByConnId: Record<string, Player>`
  - `pellets: Pellet[]`
  - `rngSeed: number`
  - `lastMoveAtByConnId: Record<string, number>`
  - `lastSeenAtByConnId: Record<string, number>`
- Player:
  - `connId: string` (server-side identity)
  - `name: string`
  - `x, y: number`
  - `radius: number`
  - `color: string`
  - `score: number`
- Lifecycle:
  - `onConnect`: create player using `c.conn.id` as key; spawn at random safe position.
  - `onDisconnect`: remove player; if empty, call matchmaker `roomClosed` then `c.destroy()`.
  - `onWake`: start a tick loop (10Hz) to spawn pellets and resolve collisions.
- Actions:
  - `setName(name: string)`
  - `move(update: { x: number; y: number; clientAt: number })`
    - Client-authoritative movement.
    - Server clamps delta by max speed using `clientAt` (or server time) and rate limits to 10-20Hz.
    - Server enforces world bounds.
- Events:
  - `snapshot(state)` at 10Hz (players, pellets, leaderboard summary).
  - Optional: `playerJoined`, `playerLeft`.

## Networking

- Clients never send `playerId`.
- All per-player mutations are keyed by `c.conn.id`.
- Payload size: snapshots should be capped (pellet count, max players).

## Game Rules

- Eating pellets increases radius.
- If a larger player overlaps a smaller player by a threshold, smaller is consumed and respawns.
- Pellets spawn to maintain a target density.

## Persistence

- Not required. Room state is ephemeral.

## Assets

Rendering:

- Deliberately minimal Agar.io style: raw circles for players/pellets and a simple procedural/grid background.

Kenney packs (chosen):

- `ui-pack` (HUD/buttons)
- `game-icons` (optional: icons for UI)

- Download at build time into `assets/kenney/` (gitignored).
- Vite config must fail fast if `assets/kenney/` is missing.
- `package.json` includes `assets:download` and `predev`/`prebuild` hooks.
- `vite.config.ts` checks for the expected directory/files and throws a clear error if missing.

## Security Checklist

- Identity:
  - Use `c.conn.id` as the only player identity.
  - Never accept `playerId` from client.
- Movement:
  - Clamp max delta per update.
  - Rate limit `move`.
  - Enforce bounds.
- Gameplay integrity:
  - Server owns pellet spawning and growth math.
  - Server computes collisions and scoring.

## Testing

Automated (Vitest):

- Actor-level tests for `ioMatchmaker` room selection/upsert/delete + stale cleanup.
- Actor-level tests for `ioRoom` identity (no spoofing), movement clamp/rate limit, and collision scoring rules.

Manual:

- Join 2 tabs, verify both appear and cannot rename/move each other.
- Try spamming `move` at 120Hz; server should clamp and/or reject.
- Verify rooms destroy when last player leaves and matchmaker forgets them.
