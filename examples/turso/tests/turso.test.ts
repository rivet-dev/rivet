import { createClient } from "@tursodatabase/api";
import { setupTest } from "rivetkit/test";
import { expect, test, vi } from "vitest";

// Each actor creates a real Turso database, so this test needs Turso credentials.
test.skipIf(!process.env.TURSO_PLATFORM_TOKEN)(
	"each list gets its own database, deleted with the list",
	async (ctx) => {
		const { registry, databaseName } = await import("../src/index.ts");
		const { client } = await setupTest(ctx, registry);
		const turso = createClient({
			org: process.env.TURSO_ORG as string,
			token: process.env.TURSO_PLATFORM_TOKEN as string,
		});

		const groceries = client.todoList.getOrCreate(["groceries"]);
		const chores = client.todoList.getOrCreate(["chores"]);
		const milk = await groceries.addTodo("Buy milk");
		await groceries.addTodo("Buy eggs");
		await chores.addTodo("Do laundry");
		await groceries.toggleTodo(milk.id);

		// Separate databases, so both lists number their rows from 1.
		expect(await groceries.listTodos()).toMatchObject([
			{ id: 1, title: "Buy milk", completed: true },
			{ id: 2, title: "Buy eggs", completed: false },
		]);
		expect(await chores.listTodos()).toMatchObject([
			{ id: 1, title: "Do laundry" },
		]);

		const groceriesDb = databaseName(await groceries.resolve());
		const choresDb = databaseName(await chores.resolve());
		await expect(turso.databases.get(groceriesDb)).resolves.toBeDefined();

		await groceries.deleteList();
		await chores.deleteList();

		// Destroy is asynchronous, so poll until onDestroy has deleted both databases.
		await vi.waitFor(
			async () => {
				const names = (await turso.databases.list()).map(
					(db) => db.name,
				);
				expect(names).not.toContain(groceriesDb);
				expect(names).not.toContain(choresDb);
			},
			{ timeout: 30_000, interval: 1_000 },
		);
	},
);
