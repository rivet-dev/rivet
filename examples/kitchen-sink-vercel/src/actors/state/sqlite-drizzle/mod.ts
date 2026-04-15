import { actor } from "rivetkit";
import { db } from "rivetkit/db/drizzle";
import { eq } from "drizzle-orm";
import * as schema from "./schema.ts";
import migrations from "./drizzle/migrations.js";

const { todos } = schema;

export const sqliteDrizzleActor = actor({
	db: db({ schema, migrations }),
	actions: {
		addTodo: async (c, title: string) => {
			const result = await c.db.insert(todos).values({
				title,
				createdAt: Date.now(),
			}).returning();
			return result[0];
		},
		getTodos: async (c) => {
			return await c.db.select().from(todos).orderBy(todos.createdAt);
		},
		toggleTodo: async (c, id: number) => {
			const existing = await c.db.select().from(todos).where(eq(todos.id, id));
			if (!existing[0]) return null;
			const newCompleted = existing[0].completed ? 0 : 1;
			const result = await c.db.update(todos)
				.set({ completed: newCompleted })
				.where(eq(todos.id, id))
				.returning();
			return result[0];
		},
		deleteTodo: async (c, id: number) => {
			await c.db.delete(todos).where(eq(todos.id, id));
			return { deleted: id };
		},
	},
});
