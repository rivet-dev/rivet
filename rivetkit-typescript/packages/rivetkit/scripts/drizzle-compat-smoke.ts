import { actor } from "rivetkit";
import { db, defineConfig, integer, sqliteTable, text } from "rivetkit/db/drizzle";

const todos = sqliteTable("todos", {
	id: text("id").primaryKey(),
	title: text("title").notNull(),
	priority: integer("priority").notNull(),
});

const config = defineConfig({
	out: "./drizzle",
	schema: "./src/schema.ts",
});

const dialect: "sqlite" = config.dialect;
void dialect;

const drizzleCompatActor = actor({
	db: db({
		schema: {
			todos,
		},
	}),
	actions: {
		listTodos: async (ctx) => {
			const rows = await ctx.db.select().from(todos);
			const firstTodoId: string | undefined = rows[0]?.id;
			void firstTodoId;

			const rawRows = await ctx.db.execute<{ count: number }>(
				"SELECT COUNT(*) as count FROM todos",
			);
			const count: number | undefined = rawRows[0]?.count;
			void count;

			return rows;
		},
	},
});

void drizzleCompatActor;
