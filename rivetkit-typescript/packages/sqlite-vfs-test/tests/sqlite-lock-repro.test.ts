import { afterEach, describe, expect, it } from "vitest";
import { mkdtempSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

interface SqliteLockError extends Error {
	code?: string;
	errcode?: number;
	errstr?: string;
}

const nodeMajorVersion = Number(process.versions.node.split(".")[0] ?? "0");
const supportsNodeSqlite = Number.isFinite(nodeMajorVersion) && nodeMajorVersion >= 22;
const lockReproTest = supportsNodeSqlite ? it : it.skip;

describe("sqlite lock repro", () => {
	let tempDir: string | undefined;

	afterEach(() => {
		if (tempDir) {
			rmSync(tempDir, { recursive: true, force: true });
			tempDir = undefined;
		}
	});

	lockReproTest(
		"throws database is locked on stmt.get when another handle holds an exclusive txn",
		async () => {
			const { DatabaseSync } = await import("node:sqlite");

			tempDir = mkdtempSync(join(tmpdir(), "sqlite-lock-repro-"));
			const dbPath = join(tempDir, "actor.db");

			const writer = new DatabaseSync(dbPath);
			const reader = new DatabaseSync(dbPath);

			try {
				writer.exec(
					"CREATE TABLE IF NOT EXISTS kv (key BLOB PRIMARY KEY NOT NULL, value BLOB NOT NULL)",
				);
				writer
					.prepare("INSERT OR REPLACE INTO kv (key, value) VALUES (?, ?)")
					.run(new Uint8Array([1]), new Uint8Array([2]));

				// Prepare the statement before the lock to match the failing runtime stack.
				const readerStmt = reader.prepare(
					"SELECT value FROM kv WHERE key = ?",
				);

				writer.exec("BEGIN EXCLUSIVE");
				writer
					.prepare("INSERT OR REPLACE INTO kv (key, value) VALUES (?, ?)")
					.run(new Uint8Array([3]), new Uint8Array([4]));

				let thrown: unknown;
				try {
					readerStmt.get(new Uint8Array([1]));
				} catch (error) {
					thrown = error;
				}

				expect(thrown).toBeDefined();
				const sqliteError = thrown as SqliteLockError;
				expect(sqliteError.code).toBe("ERR_SQLITE_ERROR");
				expect(sqliteError.errcode).toBe(5);
				expect(sqliteError.errstr).toBe("database is locked");
			} finally {
				try {
					writer.exec("ROLLBACK");
				} catch {
					// Ignore rollback failures when setup failed before BEGIN EXCLUSIVE.
				}
				writer.close();
				reader.close();
			}
		},
	);
});
