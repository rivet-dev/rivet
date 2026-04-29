import { randomBytes } from "node:crypto";
import { actor } from "rivetkit";
import { db } from "rivetkit/db";

const DEFAULT_TARGET_BYTES = 50 * 1024 * 1024;
const DEFAULT_ROW_BYTES = 16 * 1024;
const DEFAULT_BATCH_ROWS = 8;
const DEFAULT_TRANSACTION_BYTES = 64 * 1024;
const READ_BATCH_ROWS = 64;
const REVERSE_PROBE_ROWS = 32 * 1024;
const PAYLOAD_TABLE = "cold_start_payload";
const REVERSE_PROBE_TABLE = "cold_start_reverse_probe";

interface WriteInput {
	targetBytes?: number;
	rowBytes?: number;
	batchRows?: number;
	transactionBytes?: number;
}

interface PayloadRow {
	min_id?: number | null;
	max_id?: number | null;
	rows: number;
	bytes: number;
	expected_bytes: number;
}

interface PayloadValueRow {
	bytes: number;
	expected_bytes: number;
}

function positiveInteger(value: number | undefined, fallback: number, name: string) {
	const resolved = value ?? fallback;
	if (!Number.isInteger(resolved) || resolved < 1) {
		throw new Error(`${name} must be a positive integer`);
	}
	return resolved;
}

function randomAsciiString(bytes: number): string {
	return randomBytes(Math.ceil(bytes / 2)).toString("hex").slice(0, bytes);
}

async function readPayloads(
	database: {
		execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
	},
	direction: "forward" | "backward" = "forward",
) {
	const t0 = performance.now();
	const [bounds] = (await database.execute(
		`
			SELECT
				MIN(id) AS min_id,
				MAX(id) AS max_id,
				COUNT(*) AS rows,
				0 AS bytes,
				0 AS expected_bytes
			FROM ${PAYLOAD_TABLE}
		`,
	)) as PayloadRow[];

	if (!bounds) throw new Error("read query returned no rows");

	let rows = 0;
	let bytes = 0;
	let expectedBytes = 0;
	let chunks = 0;
	const minId = bounds.min_id ?? 0;
	const maxId = bounds.max_id ?? 0;

	if (direction === "backward") {
		const [probeBounds] = (await database.execute(
			`
				SELECT
					MIN(id) AS min_id,
					MAX(id) AS max_id,
					COUNT(*) AS rows,
					0 AS bytes,
					0 AS expected_bytes
				FROM ${REVERSE_PROBE_TABLE}
			`,
		)) as PayloadRow[];
		if (!probeBounds) throw new Error("reverse probe query returned no rows");
		const probeMinId = probeBounds.min_id ?? 0;
		const probeMaxId = probeBounds.max_id ?? 0;

		for (
			let upperId = probeMaxId;
			upperId >= probeMinId && upperId > 0;
			upperId -= READ_BATCH_ROWS
		) {
			const lowerId = Math.max(probeMinId, upperId - READ_BATCH_ROWS + 1);
			const chunkRows = (await database.execute(
				`
					SELECT
						marker AS bytes,
						marker AS expected_bytes
					FROM ${REVERSE_PROBE_TABLE}
					WHERE id BETWEEN ? AND ?
					ORDER BY id DESC
				`,
				lowerId,
				upperId,
			)) as PayloadValueRow[];

			for (const row of chunkRows) {
				rows += 1;
				bytes += row.bytes;
				expectedBytes += row.expected_bytes;
			}
			chunks += 1;
		}

		return {
			ms: performance.now() - t0,
			ops: rows,
			rows,
			bytes,
			expectedBytes,
			chunks,
			readBatchRows: READ_BATCH_ROWS,
			direction,
		};
	}

	for (
		let lowerId = minId;
		lowerId <= maxId;
		lowerId += READ_BATCH_ROWS
	) {
		const upperId = lowerId + READ_BATCH_ROWS - 1;
		const [chunk] = (await database.execute(
			`
				SELECT
					COUNT(*) AS rows,
					COALESCE(SUM(length(payload)), 0) AS bytes,
					COALESCE(SUM(payload_bytes), 0) AS expected_bytes
				FROM ${PAYLOAD_TABLE}
				WHERE id BETWEEN ? AND ?
			`,
			lowerId,
			upperId,
		)) as PayloadRow[];
		if (!chunk) throw new Error("chunked read query returned no rows");

		rows += chunk.rows;
		bytes += chunk.bytes;
		expectedBytes += chunk.expected_bytes;
		chunks += 1;
	}

	return {
		ms: performance.now() - t0,
		ops: rows,
		rows,
		bytes,
		expectedBytes,
		chunks,
		readBatchRows: READ_BATCH_ROWS,
	};
}

export const sqliteColdStartBench = actor({
	options: {
		actionTimeout: 600_000,
	},
	db: db({
		onMigrate: async (database) => {
			await database.execute(`
				CREATE TABLE IF NOT EXISTS cold_start_payload (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					payload TEXT NOT NULL,
					payload_bytes INTEGER NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS cold_start_reverse_probe (
					id INTEGER PRIMARY KEY,
					marker INTEGER NOT NULL
				)
			`);
		},
	}),
	actions: {
		reset: async (c) => {
			await c.db.execute(`DELETE FROM ${PAYLOAD_TABLE}`);
			await c.db.execute(`DELETE FROM ${REVERSE_PROBE_TABLE}`);
			return { ok: true };
		},

		writeRandomStrings: async (c, input: WriteInput = {}) => {
			const targetBytes = positiveInteger(
				input.targetBytes,
				DEFAULT_TARGET_BYTES,
				"targetBytes",
			);
			const rowBytes = positiveInteger(input.rowBytes, DEFAULT_ROW_BYTES, "rowBytes");
			const batchRows = positiveInteger(
				input.batchRows,
				DEFAULT_BATCH_ROWS,
				"batchRows",
			);
			const transactionBytes = positiveInteger(
				input.transactionBytes,
				DEFAULT_TRANSACTION_BYTES,
				"transactionBytes",
			);
			const createdAt = Date.now();
			let remainingBytes = targetBytes;
			let rows = 0;
			let transactions = 0;
			let randomStringMs = 0;
			let sqliteInsertMs = 0;
			let commitMs = 0;
			let inTransaction = false;

			const wallT0 = performance.now();
			try {
				while (remainingBytes > 0) {
					let transactionRemainingBytes = Math.min(
						transactionBytes,
						remainingBytes,
					);
					await c.db.execute("BEGIN");
					inTransaction = true;
					transactions += 1;

					while (transactionRemainingBytes > 0) {
						const placeholders: string[] = [];
						const args: unknown[] = [];
						const generateT0 = performance.now();

						for (
							let batchIndex = 0;
							batchIndex < batchRows &&
							transactionRemainingBytes > 0 &&
							remainingBytes > 0;
							batchIndex += 1
						) {
							const payloadBytes = Math.min(
								rowBytes,
								transactionRemainingBytes,
								remainingBytes,
							);
							placeholders.push("(?, ?, ?)");
							args.push(
								randomAsciiString(payloadBytes),
								payloadBytes,
								createdAt + rows,
							);
							transactionRemainingBytes -= payloadBytes;
							remainingBytes -= payloadBytes;
							rows += 1;
						}

						randomStringMs += performance.now() - generateT0;
						const insertT0 = performance.now();
						await c.db.execute(
							`INSERT INTO ${PAYLOAD_TABLE} (payload, payload_bytes, created_at) VALUES ${placeholders.join(", ")}`,
							...args,
						);
						sqliteInsertMs += performance.now() - insertT0;
					}

					const commitT0 = performance.now();
					await c.db.execute("COMMIT");
					commitMs += performance.now() - commitT0;
					inTransaction = false;
				}

				await c.db.execute("BEGIN");
				inTransaction = true;
				for (
					let lowerId = 1;
					lowerId <= REVERSE_PROBE_ROWS;
					lowerId += 256
				) {
					const upperId = Math.min(REVERSE_PROBE_ROWS, lowerId + 255);
					const placeholders: string[] = [];
					const args: unknown[] = [];
					for (let id = lowerId; id <= upperId; id += 1) {
						placeholders.push("(?, ?)");
						args.push(id, 1);
					}
					const insertT0 = performance.now();
					await c.db.execute(
						`INSERT INTO ${REVERSE_PROBE_TABLE} (id, marker) VALUES ${placeholders.join(", ")}`,
						...args,
					);
					sqliteInsertMs += performance.now() - insertT0;
				}
				const reverseCommitT0 = performance.now();
				await c.db.execute("COMMIT");
				commitMs += performance.now() - reverseCommitT0;
				inTransaction = false;

				return {
					ms: sqliteInsertMs + commitMs,
					writeWallMs: performance.now() - wallT0,
					randomStringMs,
					sqliteInsertMs,
					commitMs,
					ops: rows,
					rows,
					transactions,
					bytes: targetBytes,
					rowBytes,
					batchRows,
					transactionBytes,
					reverseProbeRows: REVERSE_PROBE_ROWS,
				};
			} catch (err) {
				if (inTransaction) {
					await c.db.execute("ROLLBACK");
				}
				throw err;
			}
		},

		readAll: async (c) => {
			return readPayloads(c.db);
		},

		readAllReverse: async (c) => {
			return readPayloads(c.db, "backward");
		},

		wakeSqlite: async (c) => {
			const t0 = performance.now();
			const [row] = (await c.db.execute(
				`SELECT COUNT(*) AS rows FROM ${PAYLOAD_TABLE} WHERE id = -1`,
			)) as Array<{ rows: number }>;
			return {
				ms: performance.now() - t0,
				rows: row?.rows ?? 0,
			};
		},

		goToSleep: (c) => {
			c.sleep();
			return { ok: true };
		},
	},
});
