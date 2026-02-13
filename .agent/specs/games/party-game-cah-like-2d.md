# Party Game (Cards Against Humanity-Like) (2D)

## Summary

A Jackbox-style party game with a lobby code that players join from their phones/browsers. Each round: a prompt is shown, players submit a card, a judge picks the funniest, and scores update.

- Frontend: React (UI-heavy) + minimal canvas (optional for flair).
- Backend: a single lobby actor per room.
- Deck: small and PG.

## Goals

- Demonstrate a lobby actor pattern (one actor per party room).
- Simple real-time state sync with events.
- Secure submissions: players can only submit for themselves; judge rotation enforced by server.

## Non-goals

- Large deck tooling.
- Content moderation.

## UX

- Create or join lobby by code.
- Lobby shows player list and a Start button.
- Round flow:
  - Everyone sees a prompt.
  - Each non-judge player picks one card from their hand and submits.
  - When all submitted (or timeout), judge sees submissions and picks a winner.
  - Scores update and judge rotates.

## Actors

### `partyLobby` (data)

- Key: `[code]`.
- State:
  - `phase: "lobby" | "submitting" | "judging" | "reveal"`
  - `playersByConnId: Record<string, { connId: string; name: string; score: number }>`
  - `judgeConnId: string | null`
  - `round: number`
  - `prompt: string | null`
  - `handsByConnId: Record<string, string[]>` (never broadcast)
  - `submissionsByConnId: Record<string, { card: string }>` (never broadcast)
  - `deckPrompts: string[]` (small PG list)
  - `deckAnswers: string[]` (small PG list)
  - `lastActionAtByConnId: Record<string, number>`
- Lifecycle:
  - `onConnect`: add player using `c.conn.id`, deal a hand.
  - `onDisconnect`: remove player; if lobby empty, `c.destroy()`.
- Actions:
  - `setName(name: string)`
  - `getMyHand(): string[]`:
    - Returns the caller hand only (keyed by `c.conn.id`).
  - `startGame()`:
    - Only allowed if `phase == "lobby"` and `players >= MIN_PLAYERS`.
    - Pick initial judge and start first round.
  - `submitCard(card: string)`:
    - Only allowed if caller is not judge and `phase == "submitting"`.
    - Validate card is in caller hand.
    - Remove card from hand and record submission.
  - `pickWinner(winnerConnId: string)`:
    - Only allowed if caller is judge and `phase == "judging"`.
    - Validate winnerConnId is one of the submitters.
    - Increment score, rotate judge, start next round.
- Events:
  - `lobbyState(state)` should not include hands or per-conn submissions.
  - During judging, emit a judge-only event like `judgeSubmissions({ cards: string[] })` (shuffled, anonymized).
  - On reveal, emit `roundReveal({ winnerConnId, submissions: Array<{ connId, card }> })`.

## Matchmaking

- Minimal: a client creates a random `code` and `partyLobby.create([code])`.
- Optional: add `partyMatchmaker` to allocate codes; not required.

## Assets

Kenney packs (chosen):

- `ui-pack` (UI shell)
- `boardgame-pack` (boardgame-style visuals)
- `playing-cards-pack` (card visuals)

- Download at build time into `assets/kenney/` (gitignored).
- Vite config must fail fast if `assets/kenney/` is missing.
- `package.json` includes `assets:download` and `predev`/`prebuild` hooks.
- `vite.config.ts` checks for the expected directory/files and throws a clear error if missing.

## Security Checklist

- Identity: `c.conn.id` is the only player id.
- Authorization:
  - Only judge can call `pickWinner`.
  - Only non-judge can call `submitCard`.
  - Submissions must be from caller hand.
- Rate limiting: prevent spam of submit/start.

## Testing

Automated (Vitest):

- Actor-level tests for lobby phase transitions and judge rotation.
- Actor-level tests for authorization (only judge can pick; only non-judge can submit).
- Actor-level tests for `getMyHand()` returning only the caller hand and that hands/submissions are never broadcast in `lobbyState`.

Manual:

- 3 clients: verify judge rotates and only judge can pick.
- Try submitting a card not in hand; server rejects.
- Disconnect judge mid-round; server chooses a new judge.
