import { integer, sqliteTable, text } from "rivetkit/db/drizzle";

export const usersTable = sqliteTable("users_table", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	name: text("name").notNull(),
	age: integer("age").notNull(),
	email: text("email").notNull().unique(),
});
