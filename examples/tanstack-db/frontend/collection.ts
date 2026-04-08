import { rivetCollectionOptions } from "@rivetkit/tanstack-db-collection";
import { createCollection } from "@tanstack/react-db";
import { createClient } from "rivetkit/client";
import type { registry } from "../src/actors.ts";
import type { Todo } from "../src/actors.ts";

export type { Todo };

const client = createClient<typeof registry>(
	`${window.location.origin}/api/rivet`,
);

/**
 * The TanStack DB collection for todos, backed by a Rivet Actor.
 *
 * `rivetCollectionOptions` handles the full sync lifecycle:
 * 1. Connects to the actor via `getHandle`.
 * 2. Seeds the collection with all existing todos from `getInitial`.
 * 3. Applies real-time delta updates from the actor's `change` broadcast.
 * 4. Routes optimistic mutations back to the actor via the `onInsert`,
 *    `onUpdate`, and `onDelete` handlers.
 */
export const todoCollection = createCollection<Todo, string>(
	rivetCollectionOptions({
		id: "todos",
		getHandle: () => client.todoList.getOrCreate(["default"]),
		getInitial: (conn) => conn.getTodos(),
		changeEvent: "change",
		getKey: (item) => item.id,

		onInsert: async ({ transaction, conn }) => {
			for (const mut of transaction.mutations) {
				await conn.addTodo(
					mut.modified.id,
					mut.modified.title,
					mut.modified.created_at,
				);
			}
		},

		onUpdate: async ({ transaction, conn }) => {
			for (const mut of transaction.mutations) {
				await conn.toggleTodo(mut.key as string);
			}
		},

		onDelete: async ({ transaction, conn }) => {
			for (const mut of transaction.mutations) {
				await conn.deleteTodo(mut.key as string);
			}
		},
	}),
);
