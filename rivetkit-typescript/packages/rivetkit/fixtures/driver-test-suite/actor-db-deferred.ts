import { actor } from "rivetkit";
import { db } from "@/common/database/mod";

const deferredDatabase = () =>
	db({
		commitMode: "deferred",
		onMigrate: async (database) => {
			await database.execute(
				"CREATE TABLE IF NOT EXISTS deferred_items (id INTEGER PRIMARY KEY AUTOINCREMENT, value TEXT NOT NULL)",
			);
		},
	});

export const dbActorDeferred = actor({
	db: deferredDatabase(),
	actions: {
		writeAndRead: (c, value: string) => {
			c.db.executeSync(
				"INSERT INTO deferred_items(value) VALUES (?)",
				value,
			);
			return c.db.executeSync<{ value: string }>(
				"SELECT value FROM deferred_items ORDER BY id",
			);
		},
		write: (c, value: string) => {
			const before = c.db.commitSeq();
			c.db.executeSync(
				"INSERT INTO deferred_items(value) VALUES (?)",
				value,
			);
			return {
				before,
				after: c.db.commitSeq(),
				flushed: c.db.flushedSeq(),
			};
		},
		readSequence: (c) => {
			const before = c.db.commitSeq();
			c.db.executeSync("SELECT COUNT(*) FROM deferred_items");
			return { before, after: c.db.commitSeq() };
		},
		waitForFlush: async (c, seq?: number) => {
			await c.db.waitForFlush(seq);
			return { commit: c.db.commitSeq(), flushed: c.db.flushedSeq() };
		},
		waitSnapshotThenWrite: async (c, value: string) => {
			const captured = c.db.commitSeq();
			const waiting = c.db.waitForFlush(captured);
			const later = c.db.beginTransactionSync({ name: "snapshot-later" });
			later.executeSync(
				"INSERT INTO deferred_items(value) VALUES (?)",
				value,
			);
			await waiting;
			const flushedAtEarlierWait = c.db.flushedSeq();
			const after = later.commitSync();
			return { captured, after, flushedAtEarlierWait };
		},
		readonlyMetadata: (c) => ({
			select: c.db.executeSyncRaw("SELECT 1").readonly,
			insert: c.db.executeSyncRaw(
				"INSERT INTO deferred_items(value) VALUES ('metadata')",
			).readonly,
			ddl: c.db.executeSyncRaw(
				"CREATE TABLE IF NOT EXISTS deferred_metadata (id INTEGER)",
			).readonly,
		}),
		handleTransaction: async (c, value: string) => {
			const handle = c.db.beginTransactionSync({ name: "deferred-turn" });
			handle.executeSyncRaw(
				"INSERT INTO deferred_items(value) VALUES (?)",
				value,
			);
			let baseSyncError = "";
			try {
				c.db.executeSync("SELECT 1");
			} catch (error) {
				baseSyncError =
					error instanceof Error ? error.message : String(error);
			}
			const queued = c.db.execute<{ count: number }>(
				"SELECT COUNT(*) AS count FROM deferred_items",
			);
			let resolvedWhileOpen = false;
			void queued.then(() => {
				resolvedWhileOpen = handle.isOpen;
			});
			let committedSequence: number | null = null;
			await new Promise<void>((resolve) =>
				setImmediate(() => {
					committedSequence = handle.commitSync();
					resolve();
				}),
			);
			const rows = await queued;
			if (committedSequence !== null) {
				await c.db.waitForFlush(committedSequence);
			}
			return {
				baseSyncError,
				isOpen: handle.isOpen,
				resolvedWhileOpen,
				count: rows[0]?.count ?? 0,
				committedSequence,
				sequence: c.db.commitSeq(),
			};
		},
		readOnlyHandle: (c) => {
			const handle = c.db.beginTransactionSync({
				name: "read-only-turn",
			});
			handle.executeSync("SELECT COUNT(*) FROM deferred_items");
			return handle.commitSync();
		},
		values: (c) =>
			c.db.executeSync<{ value: string }>(
				"SELECT value FROM deferred_items ORDER BY id",
			),
	},
});

export const sleepDbActorDeferred = actor({
	db: deferredDatabase(),
	actions: {
		writeAndRead: (c, value: string) => {
			c.db.executeSync(
				"INSERT INTO deferred_items(value) VALUES (?)",
				value,
			);
			return c.db.executeSync<{ value: string }>(
				"SELECT value FROM deferred_items ORDER BY id",
			);
		},
		waitForFlush: async (c) => {
			await c.db.waitForFlush();
			return { commit: c.db.commitSeq(), flushed: c.db.flushedSeq() };
		},
		writeAndFlush: async (c, value: string) => {
			c.db.executeSync(
				"INSERT INTO deferred_items(value) VALUES (?)",
				value,
			);
			await c.db.waitForFlush();
			return c.db.commitSeq();
		},
		triggerSleep: (c) => c.sleep(),
		values: (c) =>
			c.db.executeSync<{ value: string }>(
				"SELECT value FROM deferred_items ORDER BY id",
			),
		sequence: (c) => c.db.commitSeq(),
	},
	options: { sleepTimeout: 100 },
});
