import { describe, expect, test } from "vitest";
import {
	getDriverMatrixCells,
	SQLITE_DRIVER_MATRIX_OPTIONS,
} from "./shared-matrix";

describe("driver matrix cells", () => {
	function withEnv<T>(
		values: Record<string, string | undefined>,
		callback: () => T,
	): T {
		const previous = new Map<string, string | undefined>();
		for (const key of Object.keys(values)) {
			previous.set(key, process.env[key]);
			const value = values[key];
			if (value === undefined) {
				delete process.env[key];
			} else {
				process.env[key] = value;
			}
		}
		try {
			return callback();
		} finally {
			for (const [key, value] of previous) {
				if (value === undefined) {
					delete process.env[key];
				} else {
					process.env[key] = value;
				}
			}
		}
	}

	test("excludes wasm with local SQLite from the normal matrix", () => {
		const cells = getDriverMatrixCells(SQLITE_DRIVER_MATRIX_OPTIONS);

		expect(
			cells.some(
				(cell) =>
					cell.runtime === "wasm" && cell.sqliteBackend === "local",
			),
		).toBe(false);
		expect(
			cells.some(
				(cell) =>
					cell.runtime === "wasm" &&
					cell.sqliteBackend === "remote" &&
					cell.skipReason === undefined,
			),
		).toBe(true);
	});

	test("keeps the expected SQLite driver matrix cells", () => {
		const cells = getDriverMatrixCells(SQLITE_DRIVER_MATRIX_OPTIONS);

		expect(
			cells.map(
				(cell) =>
					`${cell.runtime}/${cell.sqliteBackend}/${cell.encoding}`,
			),
		).toEqual([
			"native/local/bare",
			"native/local/cbor",
			"native/local/json",
			"native/remote/bare",
			"native/remote/cbor",
			"native/remote/json",
			"wasm/remote/bare",
			"wasm/remote/cbor",
			"wasm/remote/json",
		]);
	});

	test("defaults to both runnable runtime pairs", () => {
		const cells = getDriverMatrixCells({ encodings: ["bare"] });

		expect(
			cells.map(
				(cell) =>
					`${cell.runtime}/${cell.sqliteBackend}/${cell.encoding}`,
			),
		).toEqual([
			"native/local/bare",
			"native/remote/bare",
			"wasm/remote/bare",
		]);
	});

	test("honors driver matrix env filters", () => {
		const cells = withEnv(
			{
				RIVETKIT_DRIVER_TEST_RUNTIME: "native",
				RIVETKIT_DRIVER_TEST_SQLITE: "local",
				RIVETKIT_DRIVER_TEST_ENCODING: "bare",
			},
			() => getDriverMatrixCells(),
		);

		expect(
			cells.map(
				(cell) =>
					`${cell.runtime}/${cell.sqliteBackend}/${cell.encoding}`,
			),
		).toEqual(["native/local/bare"]);
	});

	test("applies driver matrix env filters to explicit suite options", () => {
		const cells = withEnv(
			{
				RIVETKIT_DRIVER_TEST_RUNTIME: "native",
				RIVETKIT_DRIVER_TEST_SQLITE: "local",
				RIVETKIT_DRIVER_TEST_ENCODING: "bare",
			},
			() => getDriverMatrixCells(SQLITE_DRIVER_MATRIX_OPTIONS),
		);

		expect(
			cells.map(
				(cell) =>
					`${cell.runtime}/${cell.sqliteBackend}/${cell.encoding}`,
			),
		).toEqual(["native/local/bare"]);
	});

	test("rejects invalid driver matrix env filters", () => {
		expect(() =>
			withEnv(
				{
					RIVETKIT_DRIVER_TEST_RUNTIME: "native,browser",
				},
				() => getDriverMatrixCells(),
			),
		).toThrow(/invalid RIVETKIT_DRIVER_TEST_RUNTIME value/);
	});
});
