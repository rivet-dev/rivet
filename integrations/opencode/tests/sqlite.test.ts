import { Effect } from "effect";
import { SqlClient } from "effect/unstable/sql";
import { describe, expect, it } from "vitest";
import { sqliteLayer } from "../src/sqlite.js";
import { sqliteFixture } from "./sqlite-fixture.js";

describe("Rivet SQLite Effect adapter", () => {
	it("commits and rolls back through the actor transaction handle", async () => {
		const { db } = sqliteFixture();
		try {
			await Effect.runPromise(
				Effect.gen(function* () {
					const sql = yield* SqlClient.SqlClient;
					yield* sql`CREATE TABLE example (id INTEGER PRIMARY KEY, value TEXT)`;
					yield* sql.withTransaction(
						sql`INSERT INTO example VALUES (1, 'committed')`,
					);
					const failed = yield* sql
						.withTransaction(
							Effect.gen(function* () {
								yield* sql`INSERT INTO example VALUES (2, 'rolled back')`;
								return yield* Effect.fail("test failure");
							}),
						)
						.pipe(Effect.result);
					expect(failed._tag).toBe("Failure");
					expect(yield* sql`SELECT * FROM example`).toEqual([
						{ id: 1, value: "committed" },
					]);
					expect(
						yield* sql.unsafe("SELECT a.id, b.id FROM example a JOIN example b")
							.values,
					).toEqual([[1, 1]]);
				}).pipe(Effect.provide(sqliteLayer(db))),
			);
		} finally {
			await db.close();
		}
	});
});
