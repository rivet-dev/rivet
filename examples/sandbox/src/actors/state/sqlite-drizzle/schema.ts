import { integer, sqliteTable, text } from "rivetkit/db/drizzle";

export const todos = sqliteTable("todos", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	title: text("title").notNull(),
	completed: integer("completed").default(0),
	createdAt: integer("created_at").notNull(),
});
