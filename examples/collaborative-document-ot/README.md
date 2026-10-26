# Collaborative Document (OT)

A shared plain text editor that uses one Rivet Actor per document to order edits with operational transformation.

## Getting Started

```sh
git clone https://github.com/rivet-dev/rivet.git
cd rivet/examples/collaborative-document-ot
npm install
npm run dev
```

## Features

- **Single sequencer per document**: Each `document` actor assigns every edit the next revision
- **Operational transformation**: Edits based on an older revision are transformed against everything committed since
- **Operation log in SQLite**: Every transformed operation and the latest text are stored in the actor's SQLite database
- **Optimistic client**: The browser applies edits right away, keeps one operation in flight, and rebases pending edits on remote ones

## Implementation

- **Insert and delete transform**: [`src/ot.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/collaborative-document-ot/src/ot.ts)
- **Document actor**: [`src/index.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/collaborative-document-ot/src/index.ts)
- **Client sync**: [`frontend/sync.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/collaborative-document-ot/frontend/sync.ts)
- **React editor**: [`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/collaborative-document-ot/frontend/App.tsx)

## Resources

Read more about [SQLite](/docs/actors/sqlite), [events](/docs/actors/events), and [actions](/docs/actors/actions).

## License

MIT
