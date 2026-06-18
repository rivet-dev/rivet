import { describe, expect, test } from "vitest";
import { BRIDGE_RIVET_ERROR_PREFIX, RivetError } from "../../actor/errors";
import {
	type JsNativeDatabaseLike,
	wrapJsNativeDatabase,
} from "./native-database";

type NativeParams = Parameters<JsNativeDatabaseLike["execute"]>[1];
type NativeExecuteResult = Awaited<ReturnType<JsNativeDatabaseLike["execute"]>>;

function deferred<T>() {
	let resolve!: (value: T) => void;
	let reject!: (error: unknown) => void;
	const promise = new Promise<T>((resolvePromise, rejectPromise) => {
		resolve = resolvePromise;
		reject = rejectPromise;
	});
	return { promise, resolve, reject };
}

class FakeNativeDatabase implements JsNativeDatabaseLike {
	active = 0;
	maxActive = 0;
	closed = false;
	kvError: string | null = null;
	executeCalls: { sql: string; params?: NativeParams; write: boolean }[] = [];
	#pending: ReturnType<typeof deferred<NativeExecuteResult>>[] = [];

	async exec() {
		return { columns: [], rows: [] };
	}

	async execute(sql: string, params?: NativeParams) {
		return await this.#startExecute(sql, params, false);
	}

	async query(sql: string, params?: NativeParams) {
		const { columns, rows } = await this.execute(sql, params);
		return { columns, rows };
	}

	async run(sql: string, params?: NativeParams) {
		const { changes } = await this.execute(sql, params);
		return { changes };
	}

	takeLastKvError() {
		return this.kvError;
	}

	async close() {
		this.closed = true;
	}

	resolveNext(result: Partial<NativeExecuteResult> = {}) {
		const pending = this.#pending.shift();
		if (!pending) {
			throw new Error("no pending native execute call");
		}
		pending.resolve({
			columns: [],
			rows: [],
			changes: 0,
			lastInsertRowId: null,
			...result,
		});
	}

	rejectNext(error: unknown) {
		const pending = this.#pending.shift();
		if (!pending) {
			throw new Error("no pending native execute call");
		}
		pending.reject(error);
	}

	async #startExecute(
		sql: string,
		params: NativeParams,
		write: boolean,
	): Promise<NativeExecuteResult> {
		this.executeCalls.push({ sql, params, write });
		this.active++;
		this.maxActive = Math.max(this.maxActive, this.active);
		const pending = deferred<NativeExecuteResult>();
		this.#pending.push(pending);
		try {
			return await pending.promise;
		} finally {
			this.active--;
		}
	}
}

describe("wrapJsNativeDatabase", () => {
	test("admits Promise.all read queries concurrently", async () => {
		const native = new FakeNativeDatabase();
		const db = wrapJsNativeDatabase(native);

		const first = db.query("SELECT 1");
		const second = db.query("SELECT 2");

		expect(native.maxActive).toBe(2);
		native.resolveNext({ columns: ["value"], rows: [[1]] });
		native.resolveNext({ columns: ["value"], rows: [[2]] });

		await expect(first).resolves.toEqual({
			columns: ["value"],
			rows: [[1]],
		});
		await expect(second).resolves.toEqual({
			columns: ["value"],
			rows: [[2]],
		});
	});

	test("normalizes supported sqlite bind values", async () => {
		const native = new FakeNativeDatabase();
		const db = wrapJsNativeDatabase(native);
		const blob = new Uint8Array([1, 2, 3]);

		const query = db.query("SELECT ?, ?, ?, ?, ?, ?, ?", [
			1n,
			true,
			"text",
			1.5,
			null,
			undefined,
			blob,
		]);

		expect(native.executeCalls[0]?.params).toEqual([
			{ kind: "int", intValue: 1 },
			{ kind: "int", intValue: 1 },
			{ kind: "text", textValue: "text" },
			{ kind: "float", floatValue: 1.5 },
			{ kind: "null" },
			{ kind: "null" },
			{ kind: "blob", blobValue: Buffer.from(blob) },
		]);

		native.resolveNext({ columns: ["value"], rows: [[1]] });

		await expect(query).resolves.toEqual({
			columns: ["value"],
			rows: [[1]],
		});
	});

	test("returns native execute metadata", async () => {
		const native = new FakeNativeDatabase();
		const db = wrapJsNativeDatabase(native);

		const write = db.execute("INSERT INTO test VALUES (1)");
		native.resolveNext({ changes: 1, lastInsertRowId: 7 });
		await expect(write).resolves.toMatchObject({
			changes: 1,
			lastInsertRowId: 7,
		});

		const fallback = db.execute("SELECT last_insert_rowid()");
		await expect(fallback).resolves.toMatchObject({
			rows: [[7]],
		});
	});

	test("throws structured RivetError from native kv last error", async () => {
		const native = new FakeNativeDatabase();
		native.kvError = `${BRIDGE_RIVET_ERROR_PREFIX}${JSON.stringify({
			group: "depot",
			code: "sqlite_commit_page_limit_exceeded",
			message: "SQLite transaction touched too many pages.",
			metadata: {
				dirty_page_count: 8193,
				max_dirty_pages: 8192,
				page_size_bytes: 4096,
			},
			public: true,
		})}`;
		const db = wrapJsNativeDatabase(native);

		const query = db.query("SELECT 1");
		native.rejectNext(new Error("disk I/O error"));

		await expect(query).rejects.toMatchObject({
			group: "depot",
			code: "sqlite_commit_page_limit_exceeded",
			metadata: {
				dirty_page_count: 8193,
				max_dirty_pages: 8192,
				page_size_bytes: 4096,
			},
		});
		await expect(query).rejects.toBeInstanceOf(RivetError);
	});

	test("close waits for admitted native calls and rejects new work", async () => {
		const native = new FakeNativeDatabase();
		const db = wrapJsNativeDatabase(native);

		const query = db.query("SELECT 1");
		const close = db.close();
		await Promise.resolve();

		expect(native.closed).toBe(false);
		native.resolveNext({ columns: ["value"], rows: [[1]] });

		await query;
		await close;

		expect(native.closed).toBe(true);
		await expect(db.query("SELECT 2")).rejects.toThrow(
			"Database is closed",
		);
	});
});
