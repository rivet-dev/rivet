import { integer, sqliteTable, text } from "rivetkit/db/drizzle";

export const todos = sqliteTable("todos", {
  id: integer("id").primaryKey({ autoIncrement: true }),
  title: text("title").notNull(),
  createdAt: integer("created_at").notNull(),
});

export const schema = { todos };
