import { describe, expect, it } from "vitest";
import { createSqliteVfs } from "../src/backend";
import type { KvVfsOptions } from "@rivetkit/sqlite-vfs";

const CHUNK_SIZE = 4096;

function keyToString(key: Uint8Array): string {
	return Buffer.from(key).toString("hex");
}

function createKvStore(): KvVfsOptions {
	const store = new Map<string, Uint8Array>();

	return {
		get: async (key) => {
			const value = store.get(keyToString(key));
			return value ? new Uint8Array(value) : null;
		},
		getBatch: async (keys) => {
			return keys.map((key) => {
				const value = store.get(keyToString(key));
				return value ? new Uint8Array(value) : null;
			});
		},
		put: async (key, value) => {
			store.set(keyToString(key), new Uint8Array(value));
		},
		putBatch: async (entries) => {
			for (const [key, value] of entries) {
				store.set(keyToString(key), new Uint8Array(value));
			}
		},
		deleteBatch: async (keys) => {
			for (const key of keys) {
				store.delete(keyToString(key));
			}
		},
	};
}

function toBlobValue(value: unknown): Uint8Array {
	if (value instanceof Uint8Array) {
		return new Uint8Array(value);
	}
	if (value instanceof ArrayBuffer) {
		return new Uint8Array(value);
	}
	if (typeof Buffer !== "undefined" && Buffer.isBuffer(value)) {
		return new Uint8Array(value);
	}
	throw new Error(`Expected blob value, got ${typeof value}`);
}

function createPattern(size: number, seed = 17): Uint8Array {
	const out = new Uint8Array(size);
	for (let i = 0; i < out.length; i++) {
		out[i] = (seed + i * 31) % 251;
	}
	return out;
}

function assertBytesEqual(actual: Uint8Array, expected: Uint8Array): void {
	expect(actual.length).toBe(expected.length);
	for (let i = 0; i < expected.length; i++) {
		if (actual[i] !== expected[i]) {
			throw new Error(`byte mismatch at offset ${i}: ${actual[i]} != ${expected[i]}`);
		}
	}
}

function applyPatch(base: Uint8Array, offset: number, patch: Uint8Array): Uint8Array {
	const next = new Uint8Array(base);
	next.set(patch, offset);
	return next;
}

async function createBlobTable(db: { exec: (sql: string) => Promise<void> }): Promise<void> {
	await db.exec(`
		CREATE TABLE IF NOT EXISTS blob_data (
			id INTEGER PRIMARY KEY,
			payload BLOB NOT NULL
		)
	`);
}

describe("sqlite-vfs", () => {
	it("persists data across VFS instances", async () => {
		const kvStore = createKvStore();

		const vfs = await createSqliteVfs();
		const db = await vfs.open("actor-1", kvStore);
		await db.exec(
			"CREATE TABLE IF NOT EXISTS test_data (id INTEGER PRIMARY KEY AUTOINCREMENT, value TEXT NOT NULL)",
		);
		await db.exec("INSERT INTO test_data (value) VALUES ('alpha')");
		await db.exec("INSERT INTO test_data (value) VALUES ('beta')");
		await db.close();

		const vfsReloaded = await createSqliteVfs();
		const dbReloaded = await vfsReloaded.open("actor-1", kvStore);

		const values: string[] = [];
		await dbReloaded.exec(
			"SELECT value FROM test_data ORDER BY id",
			(row) => {
				values.push(String(row[0]));
			},
		);

		expect(values).toEqual(["alpha", "beta"]);

		await dbReloaded.close();
	});

	it("handles chunk boundary payload sizes", async () => {
		const kvStore = createKvStore();
		const vfs = await createSqliteVfs();
		const db = await vfs.open("actor-chunk-boundary", kvStore);

		try {
			await createBlobTable(db);
			const sizes = [
				CHUNK_SIZE - 1,
				CHUNK_SIZE,
				CHUNK_SIZE + 1,
				2 * CHUNK_SIZE - 1,
				2 * CHUNK_SIZE,
				2 * CHUNK_SIZE + 1,
				4 * CHUNK_SIZE - 1,
				4 * CHUNK_SIZE,
				4 * CHUNK_SIZE + 1,
			];

			for (const [index, size] of sizes.entries()) {
				const payload = createPattern(size, index + 7);
				await db.run(
					"INSERT INTO blob_data (id, payload) VALUES (?, ?)",
					[index + 1, payload],
				);

				const result = await db.query(
					"SELECT payload FROM blob_data WHERE id = ?",
					[index + 1],
				);
				expect(result.rows.length).toBe(1);
				const readBack = toBlobValue(result.rows[0]?.[0]);
				assertBytesEqual(readBack, payload);
			}
		} finally {
			await db.close();
		}
	});

	it("handles unaligned overwrite across chunk boundaries", async () => {
		const kvStore = createKvStore();
		const vfs = await createSqliteVfs();
		const db = await vfs.open("actor-unaligned-overwrite", kvStore);

		try {
			await createBlobTable(db);

			const initial = createPattern(3 * CHUNK_SIZE + 211, 23);
			await db.run("INSERT INTO blob_data (id, payload) VALUES (?, ?)", [1, initial]);

			const patchOffset = CHUNK_SIZE - 137;
			const patch = createPattern(CHUNK_SIZE + 503, 91);
			const expected = applyPatch(initial, patchOffset, patch);
			await db.run("UPDATE blob_data SET payload = ? WHERE id = 1", [expected]);

			const result = await db.query("SELECT payload FROM blob_data WHERE id = 1");
			expect(result.rows.length).toBe(1);
			const readBack = toBlobValue(result.rows[0]?.[0]);
			assertBytesEqual(readBack, expected);
		} finally {
			await db.close();
		}
	});

	it("supports shrink and regrow workloads", async () => {
		const kvStore = createKvStore();
		const vfs = await createSqliteVfs();
		const db = await vfs.open("actor-shrink-regrow", kvStore);

		try {
			await db.exec("PRAGMA auto_vacuum = NONE");
			await createBlobTable(db);
			await db.exec("DELETE FROM blob_data");
			await db.exec("VACUUM");

			for (let i = 0; i < 40; i++) {
				await db.run(
					"INSERT INTO blob_data (id, payload) VALUES (?, ?)",
					[i + 1, createPattern(8192, i + 11)],
				);
			}

			const grown = await db.query("PRAGMA page_count");
			const grownPages = Number(grown.rows[0]?.[0] ?? 0);
			expect(grownPages).toBeGreaterThan(0);

			await db.exec("DELETE FROM blob_data");
			await db.exec("VACUUM");
			const shrunk = await db.query("PRAGMA page_count");
			const shrunkPages = Number(shrunk.rows[0]?.[0] ?? 0);
			expect(shrunkPages).toBeLessThanOrEqual(grownPages);

			for (let i = 0; i < 25; i++) {
				await db.run(
					"INSERT INTO blob_data (id, payload) VALUES (?, ?)",
					[i + 100, createPattern(12288, i + 41)],
				);
			}
			const regrown = await db.query("PRAGMA page_count");
			const regrownPages = Number(regrown.rows[0]?.[0] ?? 0);
			expect(regrownPages).toBeGreaterThan(shrunkPages);
		} finally {
			await db.close();
		}
	});

	it("reads sparse-like zeroblob regions as zeros", async () => {
		const kvStore = createKvStore();
		const vfs = await createSqliteVfs();
		const db = await vfs.open("actor-sparse-like", kvStore);

		try {
			await createBlobTable(db);
			const totalSize = 3 * CHUNK_SIZE;
			const patchOffset = 2 * CHUNK_SIZE + 97;
			const patch = createPattern(321, 171);

			await db.run(
				"INSERT INTO blob_data (id, payload) VALUES (1, zeroblob(?))",
				[totalSize],
			);
			const zeroBlobResult = await db.query("SELECT payload FROM blob_data WHERE id = 1");
			const baseBlob = toBlobValue(zeroBlobResult.rows[0]?.[0]);
			const expected = applyPatch(baseBlob, patchOffset, patch);
			await db.run("UPDATE blob_data SET payload = ? WHERE id = 1", [expected]);

			const result = await db.query("SELECT payload FROM blob_data WHERE id = 1");
			const blob = toBlobValue(result.rows[0]?.[0]);
			expect(blob.length).toBe(totalSize);

			for (let i = 0; i < patchOffset; i++) {
				if (blob[i] !== 0) {
					throw new Error(`expected zero at offset ${i}, got ${blob[i]}`);
				}
			}
			for (let i = 0; i < patch.length; i++) {
				expect(blob[patchOffset + i]).toBe(patch[i]);
			}
		} finally {
			await db.close();
		}
	});

	it("handles many small writes to hot and scattered rows", async () => {
		const kvStore = createKvStore();
		const vfs = await createSqliteVfs();
		const db = await vfs.open("actor-many-small-writes", kvStore);

		try {
			await db.exec(
				"CREATE TABLE IF NOT EXISTS kv_like (id INTEGER PRIMARY KEY, value TEXT NOT NULL)",
			);
			for (let i = 1; i <= 10; i++) {
				await db.run("INSERT INTO kv_like (id, value) VALUES (?, ?)", [i, "init"]);
			}

			for (let i = 0; i < 500; i++) {
				const id = (i % 10) + 1;
				await db.run("UPDATE kv_like SET value = ? WHERE id = ?", [
					`v-${i}`,
					id,
				]);
			}

			const results = await db.query("SELECT id, value FROM kv_like ORDER BY id");
			expect(results.rows.length).toBe(10);
			for (let i = 0; i < results.rows.length; i++) {
				const row = results.rows[i];
				expect(Number(row?.[0])).toBe(i + 1);
				expect(String(row?.[1])).toMatch(/^v-\d+$/);
			}
		} finally {
			await db.close();
		}
	});

	it("passes integrity checks after mixed workload and reopen", async () => {
		const kvStore = createKvStore();
		const vfs = await createSqliteVfs();
		const db = await vfs.open("actor-integrity", kvStore);

		try {
			await db.exec(
				"CREATE TABLE IF NOT EXISTS integrity_data (id INTEGER PRIMARY KEY, value TEXT NOT NULL, payload BLOB NOT NULL)",
			);
			for (let i = 0; i < 150; i++) {
				await db.run(
					"INSERT OR REPLACE INTO integrity_data (id, value, payload) VALUES (?, ?, ?)",
					[i + 1, `seed-${i}`, createPattern(2048 + (i % 7) * 97, i + 5)],
				);
			}
			for (let i = 0; i < 200; i++) {
				const id = (i % 150) + 1;
				if (i % 9 === 0) {
					await db.run("DELETE FROM integrity_data WHERE id = ?", [id]);
				} else {
					await db.run(
						"INSERT OR REPLACE INTO integrity_data (id, value, payload) VALUES (?, ?, ?)",
						[
							id,
							`upd-${i}`,
							createPattern(1024 + (i % 11) * 131, 100 + i),
						],
					);
				}
			}

			const integrityBefore = await db.query("PRAGMA integrity_check");
			expect(String(integrityBefore.rows[0]?.[0]).toLowerCase()).toBe("ok");
		} finally {
			await db.close();
		}

		const vfsReloaded = await createSqliteVfs();
		const dbReloaded = await vfsReloaded.open("actor-integrity", kvStore);
		try {
			const integrityAfter = await dbReloaded.query("PRAGMA integrity_check");
			expect(String(integrityAfter.rows[0]?.[0]).toLowerCase()).toBe("ok");
		} finally {
			await dbReloaded.close();
		}
	});
});
