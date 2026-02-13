# NPC Town (Pokemon-Style) With Vercel AI SDK (2.5D)

## Summary

A small 2.5D town where players can walk around and talk to NPCs. NPC dialog is generated using the Vercel AI SDK with OpenAI.

- Frontend: React + canvas.
- Backend: a town/world actor plus optional NPC actors.
- AI: OpenAI via Vercel AI SDK.

## Goals

- Demonstrate integrating AI-driven NPC dialog into a RivetKit game.
- Keep it simple: no streaming UI required, bounded memory, basic rate limits.
- Assert token availability at backend startup.

## Non-goals

- Fully persistent long-term NPC memories.
- Complex tool use.

## UX

- Player moves on a tile grid.
- NPCs are stationary.
- Interact key opens a dialog box.
- Player can send short messages; NPC replies.

## Token Requirement

- Backend must fail fast on startup if `OPENAI_API_KEY` is missing.

## Actors

### `npcTown` (data)

- Key: `["main"]`.
- State:
  - `playersByConnId: Record<string, { connId: string; name: string; tx: number; ty: number }>`
  - `npcs: Array<{ id: string; name: string; tx: number; ty: number; persona: string; memorySummary: string }>`
  - `lastTalkAtByConnId: Record<string, number>`
- Actions:
  - `setName(name: string)`
  - `move(update: { tx: number; ty: number; clientAt: number })`
    - Tile-based with simple clamp and rate limit.
  - `talk(req: { npcId: string; message: string }): { reply: string }`
    - Validate npc exists and caller is near npc.
    - Rate limit per connection.
    - Use Vercel AI SDK to generate reply using OpenAI.
    - Update `memorySummary` with a short bounded summary.
- Events:
  - `townSnapshot({ players, npcs })`.
  - Optional: `chat({ npcId, reply })`.

### Optional: `npc` (one actor per NPC)

- If used, key: `[npcId]`.
- Keeps persona/memory isolated; town actor routes to the correct NPC actor.

## AI Design

- Prompt inputs:
  - NPC persona.
  - NPC memory summary (bounded, short).
  - Player message.
  - A short system rule set (PG, no unsafe content, stay in character).
- Output constraints:
  - Short replies.
  - No hidden instructions.

## Assets

Kenney packs (chosen, pixel art):

- `roguelike-rpg-pack` (pixel tiles + characters)
- `ui-pack-pixel-adventure` (pixel UI)
- `input-prompts-pixel-16` (optional: pixel key prompt icons)

- Download at build time into `assets/kenney/` (gitignored).
- Vite config must fail fast if `assets/kenney/` is missing.
- `package.json` includes `assets:download` and `predev`/`prebuild` hooks.
- `vite.config.ts` checks for the expected directory/files and throws a clear error if missing.

## Security Checklist

- Identity: `c.conn.id` is the only player id.
- Location checks:
  - `talk()` requires proximity.
- Rate limits:
  - `talk()` per connection to control cost and spam.
- Input constraints:
  - Limit message length.

## Testing

Automated (Vitest):

- Actor-level tests for proximity checks and rate limiting in `talk()`.
- Actor-level tests that missing `OPENAI_API_KEY` fails fast at startup (or first talk call, depending on implementation choice).
- Actor-level tests for movement clamps.

Manual:

- Verify missing `OPENAI_API_KEY` fails at startup with a clear error.
- Verify you cannot talk to an NPC across the map.
- Verify rate limiting works.
