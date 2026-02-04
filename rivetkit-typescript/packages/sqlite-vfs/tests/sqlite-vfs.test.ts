import { describe, expect, it } from "vitest";
import { SqliteVfs, type KvVfsOptions } from "../src/index";

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

describe("sqlite-vfs", () => {
	it("persists data across VFS instances", async () => {
		const kvStore = createKvStore();

		const vfs = new SqliteVfs();
		const db = await vfs.open("actor-1", kvStore);
		await db.exec(
			"CREATE TABLE IF NOT EXISTS test_data (id INTEGER PRIMARY KEY AUTOINCREMENT, value TEXT NOT NULL)",
		);
		await db.exec("INSERT INTO test_data (value) VALUES ('alpha')");
		await db.exec("INSERT INTO test_data (value) VALUES ('beta')");
		await db.close();

		const vfsReloaded = new SqliteVfs();
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
});
