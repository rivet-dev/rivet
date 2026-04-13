import { describe, expect, test } from "vitest";
import { wrapJsNativeDatabase, type JsNativeDatabaseLike } from "./native-database";

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
});
