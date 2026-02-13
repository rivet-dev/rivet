# Multiplayer Cookbook (RivetKit)

Game-focused notes to turn into docs. Seeded from `/home/nathan/misc/rork-skill-old.md` and extended while writing game example specs.

## Architecture Patterns

- Actor per entity: player, room, chunk, match.
- Coordinator and data actors: use a coordinator (matchmaker/world index) to create/find data actors (rooms/chunks/matches).
- Sharding: split hot state by room, time window, random shard, or grid coordinate.
- Lifecycle: data actors can `c.destroy()` when empty to reduce cost.

## Matchmaking Patterns

- Open lobby (io-style):
  - Maintain a list of active rooms and route joiners to the most-full room under capacity.
  - Auto-create rooms as needed; auto-destroy when empty.
- Filled room queue (turn-based / fixed-size):
  - Put players in a queue until exactly `N` are ready.
  - Emit realtime queue/ready status events for UI.
  - Spawn a match actor when full and hand back the match key to both players.
- Mode-based matchmaking:
  - Matchmaker accepts `{ mode }` and routes into separate pools (e.g. `ffa`, `tdm`, `br`).
- Team management (simple):
  - Allow manual selection with server validation, or auto-balance on join.

## Game Loop & Tick Rates

- Turn-based games: do not run a tick loop.
- Casual realtime: ~10 ticks/sec (100ms).
- Fast-paced realtime: ~20 ticks/sec (50ms). Avoid going below 50ms unless you are intentionally paying for higher tick rates.
- Implement loops with `setInterval` started in `onWake` and tied to `c.abortSignal` so it stops cleanly on actor shutdown.
- For idle/offline progression, prefer `c.schedule.after(...)` to run a coarse recurring tick even when nobody is connected.
  - Use a coarse interval (e.g. 5-15 minutes) and apply catch-up based on `Date.now() - lastTickAt`.

## Realtime Data Model

- Prefer server-published snapshots/diffs via events for UI rendering.
- Keep events small and typed. For high-frequency updates, batch per tick.
- For canvas rendering, keep game world rendering outside React reconciliation (React for UI only).
- For party/lobby games, consider per-connection redaction (each player sees only their hand/secret info).

## Interest Management (Simple)

- Spatial partitioning via chunk actors is a simple form of interest management.
- Clients subscribe only to nearby chunks (e.g. 3x3 around the player) and render only subscribed state.
- For shooters, consider limiting what the client receives by proximity and/or field of view (optional; not required for simple examples).

## Netcode Options (Document, Even If Examples Stay Simple)

- Client-authoritative movement with caps and rate limits (smooth, simple, weaker anti-cheat).
- Server-authoritative movement from inputs (more robust, can feel less responsive without prediction).
- Interpolation/smoothing on the client (optional; can be added later when examples need it).

## Physics And Spatial Indexing

### Always Use Spatial Indexing

When entity counts can grow, do not do naive O(n^2) collision checks. Prefer using a spatial indexing library rather than implementing your own broadphase.

- AABB indexing: `rbush` (dynamic) or `flatbush` (static-ish, rebuilt occasionally)
  - Use AABBs even for circles and capsules (insert their bounding boxes).
- Point indexing: `d3-quadtree` for fast nearest/within-radius queries.

### Prefer Not Using A Full Physics Engine

Most multiplayer game examples can be implemented with:

- Kinematic movement rules (speed caps, bounds, simple collision tests).
- Simple primitives (circle/sphere/AABB/capsule) and raycasts.
- Server-owned resolution for game rules (hits, damage, pickups, building placement).

### If You Need An Engine, Recommend Rapier

If you hit the threshold where hand-rolled collisions become too complex (joints, stacked bodies, stable contact resolution, lots of dynamic bodies), use Rapier.

- 2D: `@dimforge/rapier2d`
- 3D: `@dimforge/rapier3d`

Fallback engines (use only if Rapier does not work for a practical reason):

- 2D: `planck-js` (Box2D-like), `matter-js`
- 3D: `cannon-es`, `ammo.js`

Notes:

- Physics engines are not mutually exclusive with spatial indexing. Use spatial indexing for interest management and broad queries regardless.
- Physics engines are mutually exclusive with each other in practice. Pick one engine per simulation.
- Avoid Three.js on the backend. For 3D hitscan and simple collisions, use analytic math or Rapier.
- `three-mesh-bvh` is optional and mainly useful on the client for fast raycasts against detailed static meshes. Skip it unless you need mesh raycasts.

## Security & Anti-Cheat (Keep It Simple)

### Baseline Checklist (All Examples)

- Identity:
  - Use `c.conn.id` as the authoritative identity of the caller.
  - Never accept `playerId` (or similar) from the client as the source of truth.
- Authorization:
  - Validate that the caller is allowed to mutate the target entity (room membership, turn ownership, host-only actions).
- Input validation:
  - Clamp sizes/lengths and validate enums.
  - Validate usernames (length, allowed chars; avoid weird/unbounded unicode).
- Rate limiting:
  - Per-connection rate limits for spammy actions (chat, join/leave, fire, move updates).
- State integrity:
  - Server recomputes derived state (scores, win conditions, placements).
  - Avoid client-authoritative changes to inventory/currency/leaderboard totals.

### Movement Validation (Client-Authoritative Friendly)

- Clients may send position/rotation updates for smoothness, but the server must:
  - Enforce max delta per update (speed cap) based on elapsed time.
  - Reject or clamp teleports.
  - Enforce world bounds (and basic collision if applicable).
  - Rate limit update frequency (e.g. 20Hz max).

## Rendering Tips

- Canvas:
  - Use `ctx.save()`/`ctx.restore()` and nested transforms for rotation/translation.
  - Split entities into small render helpers/classes to avoid monolithic render functions.
- Three.js:
  - Keep a minimal scene graph and update transforms per frame.
  - Keep network updates decoupled from render framerate (render at RAF, apply latest state).

## Assets (Kenney)

- Download Kenney asset packs at build time into a gitignored directory (per-example).
- Vite config should assert assets are present (fail fast with a clear message).
- Recommended pattern: `package.json` has `assets:download` and `predev`/`prebuild` hooks; `vite.config.ts` checks `assets/kenney/` via `fs.existsSync` and throws if missing.

## Persistence (When To Use SQLite)

- Actor state is fine for small ephemeral state (rooms, short-lived matches).
- Prefer SQLite (`rivetkit/db`) when state is:
  - Large or table-like (tiles/blocks/buildings/inventory).
  - Needs queries/indexes beyond key lookups.
  - Expected to persist long-term and grow over time.
- Matchmakers/coordinators may also use SQLite when their indexing state can grow large (room registries, matchmaking pools, large queues).

## NPCs / AI

- Keep NPC generation simple:
  - Assert `OPENAI_API_KEY` is present on backend startup.
  - Limit tokens and rate limit per player.
  - Keep NPC memory minimal and bounded (short summaries), or skip long-term memory in the example.

## Example Design Notes (From Rork + Rivet Compat)

- Prefer talking to actors directly from the frontend (`useActor` / `client`) instead of adding extra HTTP endpoints as a hop.
- Matchmaking should demonstrate open lobby, filled-room queue, and mode-based pools.
- Common pitfalls to guard against:
  - Missing tick loop where needed.
  - Actions that mutate other players without validating the caller.
