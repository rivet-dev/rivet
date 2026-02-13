# Realtime Multiplayer FPS (3D)

## Summary

A full 3D first-person shooter demonstrating:

- Mode-based matchmaking: FFA, TDM, Battle Royale.
- A match actor that hosts an arena, validates firing, tracks kills, and ends rounds.
- Client-authoritative movement with simple caps and rate limits (no interpolation).

Frontend: React + Three.js.

## Goals

- A single example that showcases three matchmaking pools and a mode selector.
- Fast-paced feel via client-authoritative movement updates.
- Server-owned combat rules: fire rate, hitscan validation, damage, deaths.
- Basic team management for TDM (auto-balance, optional manual selection).

## Non-goals

- Complex physics.
- Advanced lag compensation.
- Anti-cheat beyond simple caps/rate limits.

## UX

- Menu: pick mode (FFA/TDM/BR), click Play.
- In match:
  - Pointer lock + WASD + mouse look.
  - Simple weapon (hitscan rifle).
  - HUD: health, ammo (optional), scoreboard.
- End screen: winner/scoreboard, play again.

## Actors

### `fpsMatchmaker` (coordinator)

- Key: `["main"]`.
- DB: `rivetkit/db` raw SQLite for match indexing state that may grow large.
- State:
  - `pools: Record<"ffa"|"tdm"|"br", Array<{ matchId: string; players: number; updatedAt: number }>>`
- SQLite schema:
  - `matches(mode TEXT NOT NULL, match_id TEXT NOT NULL, players INTEGER NOT NULL, updated_at INTEGER NOT NULL, PRIMARY KEY(mode, match_id))`
- Actions:
  - `findMatch(mode: "ffa"|"tdm"|"br"): { matchId: string }`
    - Route to the most-full match under capacity for that mode.
    - Create a new `fpsMatch` if none available.
  - `matchHeartbeat(mode, matchId, players)`
  - `matchClosed(mode, matchId)`
  - `cleanupStaleMatches()`
    - Delete rows where `updated_at < now - STALE_TTL_MS` to handle missed `matchClosed` calls.
    - This is defensive; matches should call `matchClosed` on shutdown, but cleanup handles crashes/network failures.

### `fpsMatch` (data)

- Key: `[matchId]`.
- Config:
  - `mode: "ffa"|"tdm"|"br"`
  - `maxPlayers` (e.g. 12 for FFA/TDM, 24 for BR)
- State:
  - `phase: "lobby" | "playing" | "finished"`
  - `playersByConnId: Record<string, Player>`
  - `teamsByConnId: Record<string, "red"|"blue">` (tdm only)
  - `projectileLog` omitted (hitscan)
  - `circle` (br only): center, radius
  - `lastMoveAtByConnId: Record<string, number>`
  - `lastFireAtByConnId: Record<string, number>`
- Player:
  - `connId`, `name`
  - `pos: { x,y,z }`, `yaw`, `pitch`
  - `hp`, `alive`, `kills`, `deaths`
- Lifecycle:
  - `onWake`: start 20Hz tick for BR circle + damage + heartbeat; FFA/TDM can be mostly event-driven.
  - `onConnect`: spawn player and assign team if mode is TDM.
  - `onDisconnect`: remove player; if empty, notify matchmaker then `c.destroy()`.
- Actions:
  - `setName(name: string)`
  - `setTeam(team: "red"|"blue")` (tdm only): optional; server may override to keep balance.
  - `move(update: { pos: {x,y,z}; yaw: number; pitch: number; clientAt: number })`
    - Client-authoritative.
    - Server rate limits (e.g. 20Hz) and clamps max distance traveled since last update.
    - Server enforces arena bounds.
  - `fire(req: { yaw: number; pitch: number; clientAt: number })`
    - Server rate limits (e.g. 8-10 shots/sec).
    - Server raycasts from the shooter's last known pos/aim.
    - Server applies damage to the first hit player (excluding same-team in TDM).
  - `respawn()` (ffa/tdm only): only if dead.
- Events:
  - `snapshot(state)` at ~10-20Hz containing players (pos/aim/hp/alive) and scoreboard.
  - `killed({ killerConnId, victimConnId })`.
  - `phaseChanged({ phase })`.

## Game Rules

- Damage: fixed per shot (e.g. 25), 4 hits to kill.
- FFA: first to X kills or time limit.
- TDM: teams score kills; first to X.
- BR:
  - No respawns.
  - Circle shrinks every N seconds; outside circle takes periodic damage.
  - Last alive wins.

## Networking

- No client-provided player identifiers.
- Server events are the only way to learn other players' state.

## Assets

Kenney packs (chosen):

- `prototype-kit` (3D level geometry + props for a simple arena)
- `prototype-textures` (3D materials/textures)
- `ui-pack-sci-fi` (HUD)
- `input-prompts` (optional: control icons for tutorial overlay)

- Download at build time into `assets/kenney/` (gitignored).
- Vite config must fail fast if `assets/kenney/` is missing.
- `package.json` includes `assets:download` and `predev`/`prebuild` hooks.
- `vite.config.ts` checks for the expected directory/files and throws a clear error if missing.

## Security Checklist

- Identity: all mutations keyed by `c.conn.id`.
- Movement: clamp delta per update; rate limit.
- Combat:
  - Server owns fire rate.
  - Server computes hits; client cannot claim hits.
- Team:
  - Server enforces team selection rules.

## Testing

Automated (Vitest):

- Actor-level tests for `fpsMatchmaker` indexing/heartbeat/cleanup with SQLite-backed `matches` table.
Actor-level tests for `fpsMatch`:

- Movement rate limit and speed clamp (teleport rejection).
- Fire rate limit and server-owned hit validation.
- TDM friendly-fire prevention and team rules.
- BR circle damage and last-alive win condition.

Manual:

- Two clients: verify you cannot damage teammates in TDM.
- Attempt to call `move()` with huge teleport; server clamps/rejects.
- Attempt to spam `fire()`; server rate limits.
- BR: verify circle shrink and last-alive win.
