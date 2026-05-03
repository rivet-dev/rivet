import { describe, expect, test } from "vitest";
import {
	getDriverMatrixCells,
	SQLITE_DRIVER_MATRIX_OPTIONS,
} from "./shared-matrix";

describe("driver matrix cells", () => {
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
});
