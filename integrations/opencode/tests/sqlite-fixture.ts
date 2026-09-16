import { DatabaseSync } from "node:sqlite";
import type { RawAccess } from "rivetkit/db";

export function sqliteFixture() {
	const native = new DatabaseSync(":memory:");
	let tail = Promise.resolve();
	const locked = async <T>(fn: () => Promise<T>): Promise<T> => {
		const previous = tail;
		let release!: () => void;
		tail = new Promise<void>((resolve) => {
			release = resolve;
		});
		await previous;
		try {
			return await fn();
		} finally {
			release();
		}
	};
	const execute: RawAccess["execute"] = async (query, ...args) => {
		const statement = native.prepare(query);
		return statement.all(...(args as any[])) as any;
	};
	const tx: RawAccess = {
		execute,
		transaction: async () => {
			throw new Error("Nested transaction");
		},
		close: async () => {},
	};
	const db: RawAccess = {
		execute: (query, ...args) => locked(() => execute(query, ...args)),
		transaction: (fn) =>
			locked(async () => {
				native.exec("BEGIN IMMEDIATE");
				try {
					const result = await fn(tx);
					native.exec("COMMIT");
					return result;
				} catch (error) {
					native.exec("ROLLBACK");
					throw error;
				}
			}),
		close: async () => native.close(),
	};
	return { db, native };
}
