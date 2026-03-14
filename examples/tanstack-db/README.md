# RivetKit × TanStack DB

A proof-of-concept showing how to wire [TanStack DB](https://tanstack.com/db) to a Rivet Actor, giving you:

- **SQLite persistence** inside the actor via `rivetkit/db`
- **Real-time broadcast** to every connected client the moment data changes
- **Reactive live queries** in React with sub-millisecond updates via TanStack DB's differential dataflow engine
- **Optimistic mutations** that update the UI instantly and roll back automatically if the server rejects them

## Getting Started

```bash
pnpm install
pnpm dev
```

Open `http://localhost:5173` and add todos. Open a second tab to see changes sync in real time.

## Features

- Todos are stored durably in SQLite inside the Rivet Actor — data survives server restarts and actor hibernation.
- Every mutation (add / toggle / delete) is broadcast to all connected clients via a `change` event, eliminating polling.
- A custom TanStack DB sync function seeds the local collection on connect and applies incremental deltas on each event.
- Optimistic mutations apply immediately to the local collection; TanStack DB automatically reconciles with the server state when the actor broadcasts back the confirmed change.
- A live query with filter tabs (All / Active / Completed) demonstrates how TanStack DB re-evaluates only the affected rows rather than re-running the full query.

## Implementation

### Actor (`src/actors.ts`)

The `todoList` actor owns all persistent state through a SQLite database:

```ts
export const todoList = actor({
  db: db({ onMigrate: async (db) => { /* CREATE TABLE todos */ } }),
  events: { change: event<TodoChange>() },
  actions: {
    getTodos, addTodo, toggleTodo, deleteTodo,
  },
});
```

Each write action stores the change in SQLite and then calls `c.broadcast("change", ...)` so every connected client receives a typed `TodoChange` delta.

See [`src/actors.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/tanstack-db/src/actors.ts).

### TanStack DB collection bridge (`frontend/collection.ts`)

A module-level `createCollection` call wires TanStack DB's sync callbacks to the actor:

```ts
export const todoCollection = createCollection<Todo, string>({
  getKey: (item) => item.id,
  sync: {
    sync: ({ begin, write, commit, markReady }) => {
      syncCbs = { begin, write, commit, markReady };
      return () => { syncCbs = null; };
    },
  },
  onInsert: async ({ transaction }) => { /* actor.addTodo(...) */ },
  onUpdate: async ({ transaction }) => { /* actor.toggleTodo(...) */ },
  onDelete: async ({ transaction }) => { /* actor.deleteTodo(...) */ },
});
```

On connect, `initCollection(conn)` fetches all rows with `getTodos()` and seeds the collection via `begin/write/commit/markReady`. After that, `applyChange(event)` is called for every incoming `change` event.

See [`frontend/collection.ts`](https://github.com/rivet-dev/rivet/tree/main/examples/tanstack-db/frontend/collection.ts).

### React app (`frontend/App.tsx`)

The app uses `useActor` from `@rivetkit/react` to connect to the actor, wires up the sync bridge with `useEffect`, and subscribes to events with `useEvent`:

```tsx
const actor = useActor({ name: "todoList", key: ["default"] });

// Seed the collection once connected
useEffect(() => {
  if (!actor.connection) return;
  initCollection(actor.connection).then(() => setInitialized(true));
}, [actor.connection]);

// Apply real-time deltas from other clients
actor.useEvent("change", (change) => applyChange(change));

// Live query — re-evaluates sub-millisecond on any collection change
const { data: todos } = useLiveQuery((q) =>
  q.from({ todo: todoCollection }).orderBy(({ todo }) => todo.created_at, "desc"),
);
```

See [`frontend/App.tsx`](https://github.com/rivet-dev/rivet/tree/main/examples/tanstack-db/frontend/App.tsx).

## Resources

- [Rivet Actor state](/docs/actors/state) and [actions](/docs/actors/actions)
- [Actor events](/docs/actors/events) for real-time broadcast
- [SQLite in actors](/docs/actors/sqlite)
- [TanStack DB docs](https://tanstack.com/db/latest/docs)

## License

MIT
