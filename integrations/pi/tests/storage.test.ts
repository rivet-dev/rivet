import { DatabaseSync } from "node:sqlite";
import type { RawAccess } from "rivetkit/db";
import { describe, expect, it } from "vitest";
import {
	loadPiSession,
	migratePiActorTables,
	savePiSession,
	serializeTranscript,
} from "../src/storage.js";

function sqliteAccess(database: DatabaseSync): Pick<RawAccess, "execute"> {
	return {
		async execute<TRow extends Record<string, unknown>>(
			query: string,
			...args: unknown[]
		): Promise<TRow[]> {
			const statements = query
				.split(";")
				.map((statement) => statement.trim())
				.filter(Boolean);
			if (statements.length > 1) {
				database.exec(query);
				return [];
			}
			const statement = database.prepare(query);
			if (statement.columns().length > 0) {
				return statement.all(...(args as any[])) as TRow[];
			}
			statement.run(...(args as any[]));
			return [];
		},
	};
}

describe("Pi SQLite session storage", () => {
	it("migrates and round-trips the exact JSONL transcript and sandbox binding", async () => {
		const database = new DatabaseSync(":memory:");
		const access = sqliteAccess(database);
		await migratePiActorTables(access);
		const transcript = serializeTranscript(
			{
				type: "session",
				version: 3,
				id: "session-1",
				timestamp: "2026-09-02T00:00:00.000Z",
				cwd: "/workspace",
			},
			[],
		);
		await savePiSession(access, {
			sessionId: "session-1",
			cwd: "/workspace",
			transcript,
			sandboxBinding: { provider: "test", id: "sandbox-1" },
			settings: { steeringMode: "one-at-a-time" },
		});

		expect(await loadPiSession(access)).toEqual({
			sessionId: "session-1",
			cwd: "/workspace",
			transcript,
			sandboxBinding: { provider: "test", id: "sandbox-1" },
			settings: { steeringMode: "one-at-a-time" },
		});
		database.close();
	});
});
