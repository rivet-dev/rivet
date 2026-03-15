import type { ActorConnRaw } from "rivetkit/client";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnyConn = any;

/**
 * A message carrying a single row change from the actor's broadcast event.
 * Mirrors the TanStack DB write API:
 * - Insert / update: `{ type, value }` — the key is derived by `getKey`.
 * - Delete: `{ type: 'delete', key }` — value no longer exists.
 */
export type RivetChangeMessage<T, TKey extends string | number> =
	| { type: "insert" | "update"; value: T }
	| { type: "delete"; key: TKey };

export interface RivetCollectionOptions<
	TItem extends object,
	TKey extends string | number,
	TConn,
> {
	/**
	 * Optional stable identifier for the collection. Passed through to TanStack DB.
	 */
	id?: string;

	/**
	 * Factory that returns a typed actor handle. Called once when the collection
	 * mounts so the connection can be established. Returning the handle from a
	 * module-level client is the most common pattern:
	 *
	 * ```ts
	 * getHandle: () => client.todoList.getOrCreate(["default"])
	 * ```
	 */
	getHandle: () => { connect(): TConn };

	/**
	 * Called immediately after the actor connection opens to populate the
	 * collection with the current server-side state. The return value is used as
	 * the initial snapshot.
	 */
	getInitial: (conn: TConn) => Promise<TItem[]>;

	/**
	 * Name of the actor broadcast event that carries incremental change deltas.
	 * The event payload must conform to {@link RivetChangeMessage}.
	 */
	changeEvent: string;

	/**
	 * Derive the stable collection key from an item. Must be unique per item.
	 */
	getKey: (item: TItem) => TKey;

	/**
	 * Called when TanStack DB performs an optimistic insert. Persist the
	 * mutation to the actor here. `conn` is the live actor connection.
	 */
	onInsert?: (opts: {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		transaction: any;
		conn: TConn;
	}) => Promise<void>;

	/**
	 * Called when TanStack DB performs an optimistic update. Persist the
	 * mutation to the actor here. `conn` is the live actor connection.
	 */
	onUpdate?: (opts: {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		transaction: any;
		conn: TConn;
	}) => Promise<void>;

	/**
	 * Called when TanStack DB performs an optimistic delete. Persist the
	 * mutation to the actor here. `conn` is the live actor connection.
	 */
	onDelete?: (opts: {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		transaction: any;
		conn: TConn;
	}) => Promise<void>;
}

/**
 * Creates a TanStack DB `CollectionConfig` backed by a Rivet Actor.
 *
 * The returned config can be passed directly to `createCollection()`:
 *
 * ```ts
 * import { createCollection } from "@tanstack/react-db";
 * import { rivetCollectionOptions } from "@rivetkit/tanstack-db-collection";
 * import { createClient } from "rivetkit/client";
 * import type { registry } from "./actors.ts";
 *
 * const client = createClient<typeof registry>(`${location.origin}/api/rivet`);
 *
 * const todoCollection = createCollection(
 *   rivetCollectionOptions({
 *     id: "todos",
 *     getHandle: () => client.todoList.getOrCreate(["default"]),
 *     getInitial: (conn) => conn.getTodos(),
 *     changeEvent: "change",
 *     getKey: (item) => item.id,
 *     onInsert: async ({ transaction, conn }) => {
 *       for (const mut of transaction.mutations) {
 *         await conn.addTodo(mut.modified.id, mut.modified.title, mut.modified.created_at);
 *       }
 *     },
 *     onUpdate: async ({ transaction, conn }) => {
 *       for (const mut of transaction.mutations) {
 *         await conn.toggleTodo(mut.key);
 *       }
 *     },
 *     onDelete: async ({ transaction, conn }) => {
 *       for (const mut of transaction.mutations) {
 *         await conn.deleteTodo(mut.key);
 *       }
 *     },
 *   })
 * );
 * ```
 *
 * Sync lifecycle:
 * 1. When the collection mounts, `getHandle()` is called and the actor
 *    connection is established.
 * 2. Once connected, `getInitial(conn)` fetches the full server snapshot and
 *    seeds the collection via TanStack DB's begin/write/commit/markReady cycle.
 * 3. Every subsequent broadcast of `changeEvent` is applied as an incremental
 *    delta (begin/write/commit) so all live queries react immediately.
 * 4. When the collection unmounts, the actor connection is disposed.
 */
export function rivetCollectionOptions<
	TItem extends object,
	TKey extends string | number,
	TConn = AnyConn,
>(
	opts: RivetCollectionOptions<TItem, TKey, TConn>,
) {
	let conn: TConn | null = null;

	return {
		id: opts.id,
		getKey: opts.getKey,

		sync: {
			sync: ({ begin, write, commit, markReady }) => {
				const handle = opts.getHandle();
				const connection = handle.connect();
				conn = connection;

				const rawConn = connection as unknown as ActorConnRaw;

				const unsubOpen = rawConn.onOpen(() => {
					void (async () => {
						const items = await opts.getInitial(connection);
						begin();
						for (const item of items) {
							write({ type: "insert", value: item });
						}
						commit();
						markReady();
					})();
				});

				const unsubChange = rawConn.on(
					opts.changeEvent,
					(change: RivetChangeMessage<TItem, TKey>) => {
						begin();
						write(change as Parameters<typeof write>[0]);
						commit();
					},
				);

				return () => {
					unsubOpen();
					unsubChange();
					void rawConn.dispose();
					conn = null;
				};
			},
		},

		onInsert: opts.onInsert
			? async (ctx) => {
					if (conn) await opts.onInsert!({ transaction: ctx.transaction, conn });
				}
			: undefined,

		onUpdate: opts.onUpdate
			? async (ctx) => {
					if (conn) await opts.onUpdate!({ transaction: ctx.transaction, conn });
				}
			: undefined,

		onDelete: opts.onDelete
			? async (ctx) => {
					if (conn) await opts.onDelete!({ transaction: ctx.transaction, conn });
				}
			: undefined,
	};
}
