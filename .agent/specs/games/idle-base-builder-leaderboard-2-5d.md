# Idle Base Builder With Global Leaderboard (2.5D)

## Summary

A Clash-of-Clans-inspired idle/base builder where each player has their own world actor and the game exposes a global leaderboard across all players.

- Frontend: React + canvas (2.5D tile base).
- Backend: one actor per player world; one coordinator actor for leaderboard.

## Goals

- Demonstrate "actor per player" pattern for isolated worlds.
- Simple idle loop: gather resources over time, build/upgrade structures.
- Global leaderboard maintained by a coordinator actor.
- Keep it simple and secure: no spoofing other players, server-owned resource math.

## Non-goals

- PvP attacks/raids.
- Complex offline progression (e.g. long queues, boosts, or minute-by-minute simulation while away). This example still supports simple offline catch-up.

## Identity Assumption

- Caller identity is `c.conn.id`.
- Player identity is a stable `playerId` stored in `localStorage`.
  - The client generates a random UUID on first load and persists it.
  - The `idleWorld` actor key is `[playerId]`.
  - The actor still uses `c.conn.id` to identify the caller connection for rate limiting and per-connection events.

## UX

- Landing: enter name, start.
- Base view: grid with a few buildings.
- Actions:
  - Place building.
  - Upgrade building.
  - Collect resources.
- Sidebar: global leaderboard.

## Actors

### `idleLeaderboard` (coordinator)

- Key: `["main"]`.
- State:
  - `entries: Array<{ playerId: string; name: string; score: number; updatedAt: number }>`
- Actions:
  - `upsertEntry(entry)` called by each player world on changes.
  - `getTop(n: number)`.
- Events:
  - `leaderboardUpdated(top)`.

### `idleWorld` (data)

- Key: `[playerId]` (from `localStorage`).
- DB: `rivetkit/db` raw SQLite for persistent, structured world data (buildings/resources/last tick).
- DB usage: use `onMigrate` to create tables and `c.db.execute(...)` for reads/writes (no ORM).
- State:
  - `name: string` (cached for snapshots; source of truth can be SQLite)
  - `resources: { gold: number; wood: number }` (cached; source of truth in SQLite)
  - `buildings: Array<{ id: string; kind: "hut"|"mine"|"lumber"; level: number; tx: number; ty: number }>` (cached; source of truth in SQLite)
  - `lastTickAt: number` (cached; source of truth in SQLite)
  - `nextTickAt: number | null`
  - `lastActionAtByConnId: Record<string, number>`
- SQLite schema (per world actor):
  - `world(name TEXT, gold INTEGER, wood INTEGER, last_tick_at INTEGER)`
    - Single-row table (or a keyed row) storing aggregate world values.
  - `buildings(id TEXT PRIMARY KEY, kind TEXT, level INTEGER, tx INTEGER, ty INTEGER)`
- Lifecycle:
  - `onWake`: schedule recurring ticks with `c.schedule.after(...)` so the world progresses even when no one is connected.
  - `onConnect`: initialize if first time; ensure only the connecting `c.conn.id` can control this world.
- Actions:
  - `setName(name: string)`.
  - `placeBuilding(req: { kind; tx; ty })`:
    - Validate bounds and collisions.
    - Deduct cost.
  - `upgradeBuilding(buildingId: string)`:
    - Validate ownership (implicitly by world actor key).
    - Deduct cost and increment level.
  - `collect()` (optional): move some resources from "produced" to "available".
  - `tick()`:
    - Reads `last_tick_at` from SQLite, computes `deltaMs`, and updates `gold/wood/last_tick_at` in SQLite.
    - Optionally refreshes cached `resources/lastTickAt` for snapshots.
    - Sets `lastTickAt = Date.now()`.
    - Schedules the next tick with `c.schedule.after(TICK_INTERVAL_MS, "tick")` and sets `nextTickAt = Date.now() + TICK_INTERVAL_MS`.
- Leaderboard integration:
  - After meaningful changes (name, resources, upgrades), compute a simple score and call `idleLeaderboard.upsertEntry`.
- Events:
  - `worldSnapshot(state)`.

## Scoring

- Simple score formula, e.g. `sum(building.level) + floor(gold / 100) + floor(wood / 100)`.

## Offline Progression (Using `c.schedule`)

This example should progress while the user is offline by using scheduled ticks (`c.schedule.after`) to wake the actor periodically.

Guidelines:

- Use a coarse `TICK_INTERVAL_MS` (e.g. 5-15 minutes) to keep it cheap.
- In `onWake`, if `nextTickAt` is `null` (or clearly stale), schedule `tick()` soon and set `nextTickAt`.
- `tick()` should always schedule the next tick.

Catch-up math (inside `tick()`):

- `deltaMs = Date.now() - lastTickAt`
- `deltaMs = clamp(deltaMs, 0, MAX_OFFLINE_MS)` (e.g. cap at 24h to keep it bounded)
- Compute resource gain from building production rates (from SQLite `buildings`) for `deltaMs`, add to SQLite `world.gold/world.wood`, and set SQLite `world.last_tick_at = Date.now()`.

## Assets

Kenney packs (chosen):

- `isometric-miniature-bases` (base ground tiles)
- `isometric-miniature-farm` (buildings/props that read as a base)
- `ui-pack` (UI)
- `input-prompts` (optional: key prompt icons)

- Download at build time into `assets/kenney/` (gitignored).
- Vite config must fail fast if `assets/kenney/` is missing.
- `package.json` includes `assets:download` and `predev`/`prebuild` hooks.
- `vite.config.ts` checks for the expected directory/files and throws a clear error if missing.

## Security Checklist

- No spoofing:
  - World actor is keyed by server-known identity (`c.conn.id`).
  - Actions do not accept player ids.
- Server-owned economy:
  - Server computes resource accrual.
  - Server validates costs and persists authoritative values in SQLite.
- Rate limit:
  - Throttle spam actions (place/upgrade).

## Testing

Automated (Vitest):

- Actor-level tests for `idleWorld` raw SQLite migrations and persistence (buildings/resources/last tick).
- Actor-level tests for scheduled tick catch-up math and caps.
- Actor-level tests for economy invariants (costs validated, no negative resources).
- Actor-level tests for `idleLeaderboard` ordering and updates.

Manual:

- Two clients: worlds are isolated.
- Leaderboard updates as buildings upgrade.
- Verify offline progression: build a mine, wait (or advance time if supported), reconnect and confirm resources increased.
