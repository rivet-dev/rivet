import { describe, expect, it } from "vitest";
import { SqliteVfsPool } from "@rivetkit/sqlite-vfs";
import type { KvVfsOptions } from "@rivetkit/sqlite-vfs";

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

describe("SqliteVfsPool", () => {
	it("acquire returns a handle that can open and close a database", async () => {
		const pool = new SqliteVfsPool({ actorsPerInstance: 50 });
		const handle = await pool.acquire("actor-a");

		const kvStore = createKvStore();
		const db = await handle.open("ignored-filename", kvStore);

		await db.exec("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)");
		await db.exec("INSERT INTO test (value) VALUES ('hello')");

		const result = await db.query("SELECT value FROM test");
		expect(result.rows.length).toBe(1);
		expect(String(result.rows[0]?.[0])).toBe("hello");

		await db.close();
		await handle.destroy();
		await pool.shutdown();
	});

	it("acquire with same actorId returns the same handle (sticky assignment)", async () => {
		const pool = new SqliteVfsPool({ actorsPerInstance: 50 });

		const handle1 = await pool.acquire("actor-sticky");
		const handle2 = await pool.acquire("actor-sticky");

		expect(handle1).toBe(handle2);

		await handle1.destroy();
		await pool.shutdown();
	});

	it("release then re-acquire gets a new handle", async () => {
		const pool = new SqliteVfsPool({ actorsPerInstance: 50 });

		const handle1 = await pool.acquire("actor-reacquire");
		await handle1.destroy();

		const handle2 = await pool.acquire("actor-reacquire");
		expect(handle2).not.toBe(handle1);

		// The new handle should still work.
		const kvStore = createKvStore();
		const db = await handle2.open("test", kvStore);
		await db.exec("CREATE TABLE t (id INTEGER PRIMARY KEY)");
		await db.close();

		await handle2.destroy();
		await pool.shutdown();
	});

	it("releasing short name '1' does not close short name '10'", async () => {
		const pool = new SqliteVfsPool({ actorsPerInstance: 50 });

		// Acquire 11 actors so short names '0' through '10' are assigned.
		const handles = [];
		const actorIds = [];
		for (let i = 0; i < 11; i++) {
			const actorId = `actor-${i}`;
			actorIds.push(actorId);
			handles.push(await pool.acquire(actorId));
		}

		// Open databases for actor with short name '1' (actor-1) and
		// short name '10' (actor-10). Each gets its own KV store.
		const kvStore1 = createKvStore();
		const kvStore10 = createKvStore();

		const db1 = await handles[1]!.open("db", kvStore1);
		await db1.exec("CREATE TABLE t1 (id INTEGER PRIMARY KEY, v TEXT)");
		await db1.exec("INSERT INTO t1 (v) VALUES ('from-actor-1')");

		const db10 = await handles[10]!.open("db", kvStore10);
		await db10.exec("CREATE TABLE t10 (id INTEGER PRIMARY KEY, v TEXT)");
		await db10.exec("INSERT INTO t10 (v) VALUES ('from-actor-10')");

		// Release actor-1 (short name '1'). This should force-close only
		// databases with fileName exactly '1', not '10'.
		await handles[1]!.destroy();

		// Actor-10's database should still be usable.
		await db10.exec("INSERT INTO t10 (v) VALUES ('after-release-1')");
		const result = await db10.query("SELECT v FROM t10 ORDER BY id");
		expect(result.rows.length).toBe(2);
		expect(String(result.rows[0]?.[0])).toBe("from-actor-10");
		expect(String(result.rows[1]?.[0])).toBe("after-release-1");

		// Clean up.
		await db10.close();
		for (let i = 0; i < 11; i++) {
			if (i !== 1) {
				await handles[i]!.destroy();
			}
		}
		await pool.shutdown();
	});

	it("double destroy() on PooledSqliteHandle is idempotent", async () => {
		const pool = new SqliteVfsPool({ actorsPerInstance: 50 });
		const handle = await pool.acquire("actor-double-destroy");

		// First destroy should succeed.
		await handle.destroy();

		// Second destroy should be a no-op (no error).
		await handle.destroy();

		await pool.shutdown();
	});

	it("acquire after shutdown throws", async () => {
		const pool = new SqliteVfsPool({ actorsPerInstance: 50 });

		// Acquire and release one actor to exercise the pool.
		const handle = await pool.acquire("actor-before-shutdown");
		await handle.destroy();

		await pool.shutdown();

		await expect(pool.acquire("actor-after-shutdown")).rejects.toThrow(
			"SqliteVfsPool is shutting down",
		);
	});

	it("actorsPerInstance limit triggers new instance creation", async () => {
		const pool = new SqliteVfsPool({ actorsPerInstance: 2 });

		// Acquire 2 actors to fill the first instance.
		const handle1 = await pool.acquire("actor-limit-1");
		const handle2 = await pool.acquire("actor-limit-2");

		// The third actor should trigger a new instance.
		const handle3 = await pool.acquire("actor-limit-3");

		// All three handles should work independently.
		const kvStore1 = createKvStore();
		const kvStore2 = createKvStore();
		const kvStore3 = createKvStore();

		const db1 = await handle1.open("db", kvStore1);
		const db2 = await handle2.open("db", kvStore2);
		const db3 = await handle3.open("db", kvStore3);

		await db1.exec("CREATE TABLE t (v TEXT)");
		await db2.exec("CREATE TABLE t (v TEXT)");
		await db3.exec("CREATE TABLE t (v TEXT)");

		await db1.exec("INSERT INTO t (v) VALUES ('one')");
		await db2.exec("INSERT INTO t (v) VALUES ('two')");
		await db3.exec("INSERT INTO t (v) VALUES ('three')");

		const r1 = await db1.query("SELECT v FROM t");
		const r2 = await db2.query("SELECT v FROM t");
		const r3 = await db3.query("SELECT v FROM t");

		expect(String(r1.rows[0]?.[0])).toBe("one");
		expect(String(r2.rows[0]?.[0])).toBe("two");
		expect(String(r3.rows[0]?.[0])).toBe("three");

		await db1.close();
		await db2.close();
		await db3.close();
		await handle1.destroy();
		await handle2.destroy();
		await handle3.destroy();
		await pool.shutdown();
	});

	it("pool scales up, scales down on idle, and scales back up", async () => {
		const pool = new SqliteVfsPool({
			actorsPerInstance: 2,
			idleDestroyMs: 100,
		});

		// -- Scale up: 6 actors with limit 2 → 3 instances --
		const handles = [];
		const kvStores = [];
		for (let i = 0; i < 6; i++) {
			handles.push(await pool.acquire(`scale-actor-${i}`));
			kvStores.push(createKvStore());
		}
		expect(pool.instanceCount).toBe(3);
		expect(pool.actorCount).toBe(6);

		// Verify all 6 actors work with isolated data
		for (let i = 0; i < 6; i++) {
			const db = await handles[i]!.open("db", kvStores[i]!);
			await db.exec("CREATE TABLE t (v TEXT)");
			await db.exec(`INSERT INTO t (v) VALUES ('actor-${i}')`);
			const result = await db.query("SELECT v FROM t");
			expect(String(result.rows[0]?.[0])).toBe(`actor-${i}`);
			await db.close();
		}

		// -- Scale down: release all actors, wait for idle destroy --
		for (const handle of handles) {
			await handle.destroy();
		}
		expect(pool.actorCount).toBe(0);

		// Wait for idle timers to fire and destroy instances
		await new Promise((r) => setTimeout(r, 250));
		expect(pool.instanceCount).toBe(0);

		// -- Scale back up: 6 new actors → 3 new instances --
		const handles2 = [];
		const kvStores2 = [];
		for (let i = 0; i < 6; i++) {
			handles2.push(await pool.acquire(`scale-actor-new-${i}`));
			kvStores2.push(createKvStore());
		}
		expect(pool.instanceCount).toBe(3);
		expect(pool.actorCount).toBe(6);

		// Verify all new actors work
		for (let i = 0; i < 6; i++) {
			const db = await handles2[i]!.open("db", kvStores2[i]!);
			await db.exec("CREATE TABLE t (v TEXT)");
			await db.exec(`INSERT INTO t (v) VALUES ('new-${i}')`);
			const result = await db.query("SELECT v FROM t");
			expect(String(result.rows[0]?.[0])).toBe(`new-${i}`);
			await db.close();
		}

		for (const handle of handles2) {
			await handle.destroy();
		}
		await pool.shutdown();
	});

	it("partial scale down keeps correct instance count", async () => {
		const pool = new SqliteVfsPool({
			actorsPerInstance: 2,
			idleDestroyMs: 100,
		});

		// Fill 3 instances with 6 actors
		const handles = [];
		for (let i = 0; i < 6; i++) {
			handles.push(await pool.acquire(`partial-${i}`));
		}
		expect(pool.instanceCount).toBe(3);

		// Release 4 actors (leaving 2 in one instance)
		for (let i = 0; i < 4; i++) {
			await handles[i]!.destroy();
		}
		expect(pool.actorCount).toBe(2);

		// Wait for idle timers on empty instances
		await new Promise((r) => setTimeout(r, 250));

		// Only the instance with 2 remaining actors should survive
		expect(pool.instanceCount).toBe(1);
		expect(pool.actorCount).toBe(2);

		// Scale back up to 6 actors → should create 2 new instances
		const handles2 = [];
		for (let i = 0; i < 4; i++) {
			handles2.push(await pool.acquire(`partial-new-${i}`));
		}
		expect(pool.instanceCount).toBe(3);
		expect(pool.actorCount).toBe(6);

		// Clean up
		for (const h of [...handles.slice(4), ...handles2]) {
			await h!.destroy();
		}
		await pool.shutdown();
	});

	it("constructor rejects actorsPerInstance < 1", () => {
		expect(() => new SqliteVfsPool({ actorsPerInstance: 0 })).toThrow(
			"actorsPerInstance must be a positive integer",
		);
		expect(() => new SqliteVfsPool({ actorsPerInstance: -1 })).toThrow(
			"actorsPerInstance must be a positive integer",
		);
		expect(() => new SqliteVfsPool({ actorsPerInstance: 1.5 })).toThrow(
			"actorsPerInstance must be a positive integer",
		);
	});
});
