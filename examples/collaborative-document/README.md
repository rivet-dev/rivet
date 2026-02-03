# Collaborative Document

A shared text editor that uses Rivet Actors with Yjs for real-time CRDT sync and presence.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/collaborative-document
pnpm install
pnpm dev
```

## Features

- **Coordinator pattern**: One documentList actor per workspace indexes document actors
- **CRDT synchronization**: Document actors broadcast Yjs updates to every collaborator
- **Presence and cursors**: Awareness updates flow through actor events for live cursors
- **Durable persistence**: Yjs snapshots are stored in actor KV for crash recovery

## Implementation

The coordinator creates document actors and tracks IDs, while each document actor stores Yjs state in KV storage and broadcasts updates.

- **Coordinator and document actors**: [`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/collaborative-document/src/actors.ts)
- **React editor and presence UI**: [`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/collaborative-document/frontend/App.tsx)
- **Server entry point**: [`src/server.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/collaborative-document/src/server.ts)

## Resources

Read more about [coordinator actors](/docs/actors/design-patterns), [events](/docs/actors/events), [actions](/docs/actors/actions), and [KV storage](/docs/actors/kv).

## License

MIT
