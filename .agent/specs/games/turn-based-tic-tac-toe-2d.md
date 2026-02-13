# Turn-Based Multiplayer Tic Tac Toe (2D)

## Summary

A simple turn-based game demonstrating filled-room matchmaking (2 players), match creation, and strict server validation of turns.

- Frontend: React + canvas.
- Backend: RivetKit actors.
- Assets: Kenney UI pack (downloaded at build time, gitignored).

## Goals

- Matchmaker queues players until exactly 2 are ready.
- Match actor serves as lobby + game state machine.
- Secure turn enforcement: players cannot play out of turn or spoof the other seat.

## Non-goals

- Ranked matchmaking.
- Spectators.

## UX

- Landing: enter display name, click Find Match.
- Lobby: shows "waiting for opponent" until matched.
- Game: 3x3 grid, show whose turn, show winner/draw, rematch.

## Actors

### `tttMatchmaker` (coordinator)

- Key: `["main"]`.
- DB: `rivetkit/db` raw SQLite for a potentially large matchmaking queue.
- State:
  - `waitingConnIds: string[]`
  - `activeMatchIds: string[]` (optional)
- SQLite schema:
  - `queue(conn_id TEXT PRIMARY KEY, enqueued_at INTEGER NOT NULL)`
- Lifecycle:
  - `onMigrate`: create `queue` table.
- Actions:
  - `enqueue(): { status: "queued" } | { status: "matched"; matchId: string; seat: "x" | "o" }`
    - Uses `c.conn.id`.
    - If queue empty, enqueue and return queued.
    - If one waiting, pop and create a new `tttMatch` and return match info.
  - `cancelQueue()` removes `c.conn.id` if present.

Implementation note:

- Even if `waitingConnIds` exists in memory for convenience, the source of truth should be SQLite (`queue` table) so a restart does not lose the queue.

### `tttMatch` (data)

- Key: `[matchId]`.
- State:
  - `phase: "lobby" | "playing" | "finished"`
  - `players: { x?: { connId: string; name: string }; o?: { connId: string; name: string } }`
  - `board: Array<null | "x" | "o">` length 9
  - `turn: "x" | "o"`
  - `winner: null | "x" | "o" | "draw"`
- Lifecycle:
  - `onConnect`: if seat open, assign the connecting `c.conn.id` to the first available seat; otherwise treat as reconnect (same conn id) or reject.
  - `onDisconnect`: mark seat disconnected; optionally end match if a player leaves.
- Actions:
  - `setName(name: string)` sets name for caller's seat.
  - `play(index: number)`:
    - Identify caller seat by `c.conn.id`.
    - Validate `phase == "playing"`, `turn == callerSeat`, `board[index] == null`.
    - Apply move, check winner, advance turn.
  - `rematch()`:
    - Only allowed when `phase == "finished"` and both players connected.
- Events:
  - `stateUpdated(state)` whenever board/phase changes.

## Networking

- Client never sends seat or player id.
- Match actor derives caller seat by connection id.

## Assets

Kenney packs (chosen):

- `ui-pack` (buttons/frames)
- `input-prompts` (optional: key prompt icons)

- Download at build time into `assets/kenney/` (gitignored).
- Vite config must fail fast if `assets/kenney/` is missing.
- `package.json` includes `assets:download` and `predev`/`prebuild` hooks.
- `vite.config.ts` checks for the expected directory/files and throws a clear error if missing.

## Security Checklist

- No spoofing: `play()` determines seat by `c.conn.id`.
- Input validation: index in `[0..8]`.
- Rate limit: `play()` at most a few per second per connection.

## Testing

Automated (Vitest):

- Actor-level tests for queue behavior (enqueue/cancel/match pairing) with SQLite-backed queue.
- Actor-level tests for `tttMatch.play()` turn enforcement, invalid moves, winner/draw detection, and rematch reset.

Manual:

- Two tabs: confirm only the correct tab can play its own moves.
- Attempt double move in same turn; server rejects.
- Rematch resets state cleanly.
