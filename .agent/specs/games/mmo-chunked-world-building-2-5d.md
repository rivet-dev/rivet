# MMO Chunked World With Building (2.5D)

## Summary

A shared-world MMO-like sandbox where the world is split into grid chunks (one chunk = one actor). Players can move around and place/remove blocks that persist for everyone using SQLite storage.

- Frontend: React + canvas (2.5D top-down / light isometric).
- Backend: chunk actors keyed by chunk coords; per-chunk SQLite via `rivetkit/db` using the raw SQLite API (no ORM).
- No server-side handoff: clients choose which chunks to connect to.

## Goals

- Demonstrate spatial sharding via actors: `chunk[x,y]`.
- Shared persistent building that survives restarts.
- Simple movement and visibility limited by chunk subscription.

## Non-goals

- Seamless handoff/transfer logic between chunks.
- Complex pathfinding or physics.
- Strong anti-cheat.

## UX

- Player spawns near origin.
- View is a 2.5D tile grid.
- Player can:
  - Move.
  - Place a block (selected type) on a tile.
  - Remove a block.
- UI shows current chunk coordinates and which chunks you are subscribed to.

## World Model

- Chunk size: `CHUNK_TILES = 32` (32x32 tiles).
- World coordinate system:
  - Global tile coords `(tx, ty)`.
  - Chunk coords `(cx, cy) = (floor(tx/CHUNK_TILES), floor(ty/CHUNK_TILES))`.
  - Local coords `(lx, ly) = (tx mod CHUNK_TILES, ty mod CHUNK_TILES)`.

## Actors

### `worldIndex` (optional coordinator)

- Key: `["main"]`.
- Purpose:
  - Provide constants, optional list of active chunks, and an example of a coordinator.
- This can be omitted; clients can compute chunk keys directly.

### `worldChunk` (data)

- Key: `[cx, cy]` (as strings or numbers, but stable).
- DB: `rivetkit/db` raw SQLite used for block persistence (use `onMigrate` + `c.db.execute(...)`).
- State:
  - `playersByConnId: Record<string, { connId: string; name: string; tx: number; ty: number }>`
  - `blocks: Map<string, Block>` cached in memory (key = `${lx},${ly}`)
  - `lastMoveAtByConnId: Record<string, number>`
- Block:
  - `type: "wood"|"stone"|"grass"|...` (small enum)
  - `placedAt: number`
  - `placedByConnId: string`
- DB schema (per chunk):
  - `blocks(lx INTEGER, ly INTEGER, type TEXT, placed_at INTEGER, placed_by TEXT, PRIMARY KEY(lx,ly))`
- Lifecycle:
  - `onMigrate`: create `blocks` table.
  - `onWake`: load all blocks into memory (bounded by chunk size).
  - `onConnect`: add player to `playersByConnId`.
  - `onDisconnect`: remove player. Keep the actor warm; it will sleep automatically when idle.
- Actions:
  - `setName(name: string)`
  - `move(update: { tx: number; ty: number; clientAt: number })`
    - Client-authoritative but tile-based.
    - Server enforces max tile delta per update and bounds for this chunk's responsibility:
      - If `tx,ty` leaves the chunk, allow it but do not imply handoff; client should connect to the destination chunk separately.
  - `placeBlock(req: { tx: number; ty: number; type: BlockType })`
    - Validate tile is within this chunk (based on `tx,ty`).
    - Upsert into DB and in-memory cache.
  - `removeBlock(req: { tx: number; ty: number })`
    - Validate tile is within this chunk.
    - Delete from DB and cache.
- Events:
  - `chunkSnapshot({ cx, cy, players, blocks })` on connect and periodically.
  - `blockPlaced`, `blockRemoved`, `playerMoved`.

## Client Chunk Subscription

- Client computes the set of chunks to subscribe to based on player position, e.g. a 3x3 grid around the current chunk.
- Client maintains actor connections to those chunks and renders:
  - Blocks in all subscribed chunks.
  - Players in all subscribed chunks.

## Assets

Kenney packs (chosen):

- `isometric-blocks` (placeable blocks/tiles)
- `isometric-landscape` (terrain variety)
- `ui-pack` (UI)
- `input-prompts` (optional: key prompt icons)

- Download at build time into `assets/kenney/` (gitignored).
- Vite config must fail fast if `assets/kenney/` is missing.
- `package.json` includes `assets:download` and `predev`/`prebuild` hooks.
- `vite.config.ts` checks for the expected directory/files and throws a clear error if missing.

## Security Checklist

- Identity:
  - Players are keyed by `c.conn.id`.
- Building integrity:
  - Server validates target chunk ownership for `placeBlock/removeBlock`.
  - Server writes to SQLite; clients cannot directly change persisted data.
- Movement:
  - Clamp tile delta per update.
  - Rate limit move actions.

## Testing

Automated (Vitest):

- Actor-level tests for `worldChunk` raw SQLite migrations and persistence (place/remove survives restart).
- Actor-level tests for chunk ownership validation (reject place/remove for tiles outside the chunk).
- Actor-level tests for movement clamp/rate limit on tile updates.

Manual:

- Two clients in same chunk: place/remove blocks and see updates live.
- Refresh server and verify blocks persist.
- Move across chunk boundary: client should connect to new chunk and see blocks there.
