import { integer, sqliteTable, text } from "rivetkit/db/drizzle";

export const testData = sqliteTable("test_data", {
	id: integer("id").primaryKey({ autoIncrement: true }),
	value: text("value").notNull(),
	payload: text("payload").notNull().default(""),
	createdAt: integer("created_at").notNull(),
});

export const schema = {
	testData,
};
