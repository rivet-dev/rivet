import { Action, Actor, db } from "@rivetkit/effect";
import { Effect, Schema } from "effect";

export const GetAuthors = Action.make("GetAuthors", {
	success: Schema.Array(Schema.String),
});

export const GetForkedFrom = Action.make("GetForkedFrom", {
	success: Schema.Array(Schema.String),
});

export const Draft = Actor.make("Draft", {
	actions: [GetAuthors, GetForkedFrom],
	input: Schema.Struct({ author: Schema.String }),
});

const addAuthor = Effect.fn("Draft.addAuthor")(function* (author: string) {
	const sql = yield* Draft.sql;
	yield* sql`INSERT INTO authors (name) VALUES (${author})`;
});

export const DraftLive = Draft.toLayer(
	Effect.gen(function* () {
		const sql = yield* Draft.sql;
		const address = yield* Actor.CurrentAddress;

		return Draft.of({
			GetAuthors: () =>
				sql<{ name: string }>`SELECT name FROM authors`.pipe(
					Effect.map((rows) => rows.map((row) => row.name)),
					Effect.orDie,
				),
			// The actor IDs this draft was forked from, oldest first.
			GetForkedFrom: () =>
				Effect.succeed(address.forkChain.map((link) => link.actorId)),
		});
	}),
	{
		db: db({
			onMigrate: async (client) => {
				await client.execute(
					"CREATE TABLE IF NOT EXISTS authors (name TEXT NOT NULL)",
				);
			},
		}),
		onCreate: ({ author }) => addAuthor(author).pipe(Effect.orDie),
		// Runs in place of `onCreate` on the first start of a fork, on a copy of the source's database.
		onFork: ({ input }) => addAuthor(input.author).pipe(Effect.orDie),
	},
);

export const program = Effect.gen(function* () {
	const drafts = yield* Draft.client;

	const original = yield* drafts.create("post-1", {
		input: { author: "alice" },
	});
	const copy = yield* drafts.fork(yield* original.resolve, {
		key: "post-1-edit",
		input: { author: "bob" },
	});

	// ["alice", "bob"]: the copied row and the row of `onFork`.
	yield* Effect.log(yield* copy.GetAuthors());
	yield* Effect.log(yield* copy.GetForkedFrom());
});
