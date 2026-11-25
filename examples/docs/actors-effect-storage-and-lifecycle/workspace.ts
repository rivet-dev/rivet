import { Action, Actor, db } from "@rivetkit/effect";
import { Effect, Schema } from "effect";

export const GetOwner = Action.make("GetOwner", {
	success: Schema.String,
});

export const Workspace = Actor.make("Workspace", {
	actions: [GetOwner],
	input: Schema.Struct({ owner: Schema.String }),
});

export const WorkspaceLive = Workspace.toLayer(
	Effect.gen(function* () {
		const sql = yield* Workspace.sql;

		return Workspace.of({
			GetOwner: () =>
				sql<{ owner: string }>`SELECT owner FROM workspace`.pipe(
					Effect.map(([{ owner }]) => owner),
					Effect.orDie,
				),
		});
	}),
	{
		db: db({
			onMigrate: async (client) => {
				await client.execute(
					"CREATE TABLE IF NOT EXISTS workspace (owner TEXT NOT NULL)",
				);
			},
		}),
		// RivetKit does not keep the input, so store what later wakes need.
		onCreate: ({ owner }) =>
			Effect.gen(function* () {
				const sql = yield* Workspace.sql;
				yield* sql`INSERT INTO workspace (owner) VALUES (${owner})`;
			}).pipe(Effect.orDie),
	},
);
