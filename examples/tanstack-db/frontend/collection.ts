import { createCollection } from "@tanstack/react-db";
import type { Todo, TodoChange } from "../src/actors.ts";

export type { Todo };

// Callbacks provided by TanStack DB's sync function, stored so we can drive
// the collection from outside React (e.g. from useEffect hooks and event
// handlers).
type SyncCallbacks = {
	begin: () => void;
	// write accepts either { type, value } for insert/update (key derived via
	// getKey) or { type: 'delete', key } for deletes (TanStack DB API).
	write: (message: TodoChange) => void;
	commit: () => void;
	markReady: () => void;
};

let syncCbs: SyncCallbacks | null = null;
// The current actor connection, used by the mutation handlers.
let actorConn: ActorConnection | null = null;

// Minimal typing for the subset of the actor connection we use here.
export type ActorConnection = {
	getTodos: () => Promise<Todo[]>;
	addTodo: (id: string, title: string, createdAt: number) => Promise<Todo>;
	toggleTodo: (id: string) => Promise<Todo>;
	deleteTodo: (id: string) => Promise<void>;
};

/**
 * The single TanStack DB collection for todos.
 *
 * Data lifecycle:
 * 1. On actor connect: load all todos via `getTodos()`, seed the collection via
 *    `begin/write/commit/markReady`.
 * 2. On every `change` event broadcast by the actor: apply the delta via
 *    `begin/write/commit`.
 * 3. On local mutation: optimistic state is applied immediately; the mutation
 *    handler syncs it to the actor.  When the actor broadcasts the change back,
 *    the synced state catches up and the optimistic overlay is resolved.
 *
 * TanStack DB write API:
 * - Insert/update: `{ type, value }` — key derived by `getKey(value)`
 * - Delete: `{ type: 'delete', key }` — value no longer exists
 */
export const todoCollection = createCollection<Todo, string>({
	getKey: (item) => item.id,

	sync: {
		// The sync function is called once when the collection is first used.
		// We store the callbacks so initCollection/applyChange can drive it.
		sync: ({ begin, write, commit, markReady }) => {
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			syncCbs = { begin, write: write as (msg: TodoChange) => void, commit, markReady };
			// Return cleanup called when the collection is destroyed.
			return () => {
				syncCbs = null;
			};
		},
	},

	onInsert: async ({ transaction }) => {
		if (!actorConn) return;
		for (const mut of transaction.mutations) {
			const { id, title, created_at } = mut.modified;
			await actorConn.addTodo(id, title, created_at);
		}
	},

	onUpdate: async ({ transaction }) => {
		if (!actorConn) return;
		for (const mut of transaction.mutations) {
			await actorConn.toggleTodo(mut.key);
		}
	},

	onDelete: async ({ transaction }) => {
		if (!actorConn) return;
		for (const mut of transaction.mutations) {
			await actorConn.deleteTodo(mut.key);
		}
	},
});

/**
 * Called from React once the actor connection is ready.
 * Fetches all existing todos from the actor's SQLite database and seeds the
 * TanStack DB collection, making live queries reactive immediately.
 */
export async function initCollection(conn: ActorConnection): Promise<void> {
	actorConn = conn;

	if (!syncCbs) return;

	const todos = await conn.getTodos();

	syncCbs.begin();
	for (const todo of todos) {
		// Insert: key is derived by getKey(todo), so no `key` field needed.
		syncCbs.write({ type: "insert", value: todo });
	}
	syncCbs.commit();
	syncCbs.markReady();
}

/**
 * Called from React whenever the actor broadcasts a `change` event.
 * Applies the delta to the TanStack DB collection so all live queries react
 * immediately — without a full re-fetch.
 */
export function applyChange(change: TodoChange): void {
	if (!syncCbs) return;
	syncCbs.begin();
	syncCbs.write(change);
	syncCbs.commit();
}
