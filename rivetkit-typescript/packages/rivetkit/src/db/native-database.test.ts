import { describe, expect, test } from "vitest";
import {
	wrapJsNativeDatabase,
	type JsNativeDatabaseLike,
} from "./native-database";
import type { SqliteVfsTelemetry } from "./config";

const EMPTY_VFS_TELEMETRY: SqliteVfsTelemetry = {
	reads: {
		count: 0,
		durationUs: 0,
		requestedBytes: 0,
		returnedBytes: 0,
		shortReadCount: 0,
	},
	writes: {
		count: 0,
		durationUs: 0,
		inputBytes: 0,
		bufferedCount: 0,
		bufferedBytes: 0,
		immediateKvPutCount: 0,
		immediateKvPutBytes: 0,
	},
	syncs: {
		count: 0,
		durationUs: 0,
		metadataFlushCount: 0,
		metadataFlushBytes: 0,
	},
	atomicWrite: {
		beginCount: 0,
		commitAttemptCount: 0,
		commitSuccessCount: 0,
		commitDurationUs: 0,
		committedDirtyPagesTotal: 0,
		maxCommittedDirtyPages: 0,
		committedBufferedBytesTotal: 0,
		rollbackCount: 0,
		fastPathDirtyPagesTotal: 0,
		maxFastPathDirtyPages: 0,
		fastPathRequestBytesTotal: 0,
		maxFastPathRequestBytes: 0,
		fastPathDurationUs: 0,
		maxFastPathDurationUs: 0,
		batchCapFailureCount: 0,
		commitKvPutFailureCount: 0,
	},
	kv: {
		getCount: 0,
		getDurationUs: 0,
		getKeyCount: 0,
		getBytes: 0,
		putCount: 0,
		putDurationUs: 0,
		putKeyCount: 0,
		putBytes: 0,
		deleteCount: 0,
		deleteDurationUs: 0,
		deleteKeyCount: 0,
		deleteRangeCount: 0,
		deleteRangeDurationUs: 0,
	},
};

function createDatabase(
	overrides: Partial<JsNativeDatabaseLike> = {},
): JsNativeDatabaseLike {
	return {
		async exec() {
			return { columns: [], rows: [] };
		},
		async query() {
			return { columns: [], rows: [] };
		},
		async run() {
			return { changes: 0 };
		},
		async close() {},
		...overrides,
	};
}

describe("wrapJsNativeDatabase", () => {
	test("appends native sqlite kv errors to generic sqlite I/O failures", async () => {
		const db = wrapJsNativeDatabase(
			createDatabase({
				async run() {
					throw new Error(
						"failed to execute sqlite statement: disk I/O error",
					);
				},
				takeLastKvError() {
					return "envoy channel closed while writing sqlite page";
				},
			}),
		);

		await expect(db.run("INSERT INTO foo VALUES (1)")).rejects.toThrow(
			"failed to execute sqlite statement: disk I/O error (native sqlite kv error: envoy channel closed while writing sqlite page)",
		);
	});

	test("does not attach native sqlite kv errors to unrelated sqlite failures", async () => {
		const db = wrapJsNativeDatabase(
			createDatabase({
				async run() {
					throw new Error(
						"failed to execute sqlite statement: no such table: foo",
					);
				},
				takeLastKvError() {
					return "envoy channel closed while writing sqlite page";
				},
			}),
		);

		await expect(db.run("INSERT INTO foo VALUES (1)")).rejects.toThrow(
			"failed to execute sqlite statement: no such table: foo",
		);
	});

	test("passes through VFS telemetry helpers when the native handle exposes them", async () => {
		let resetCount = 0;
		const db = wrapJsNativeDatabase(
			createDatabase({
				async resetVfsTelemetry() {
					resetCount += 1;
				},
				async snapshotVfsTelemetry() {
					return EMPTY_VFS_TELEMETRY;
				},
			}),
		);

		await db.resetVfsTelemetry?.();
		await expect(db.snapshotVfsTelemetry?.()).resolves.toEqual(
			EMPTY_VFS_TELEMETRY,
		);
		expect(resetCount).toBe(1);
	});
});
