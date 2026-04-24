import { actor } from "rivetkit";
import { db } from "rivetkit/db";

type WorkloadMode =
	| "balanced"
	| "hot"
	| "transactions"
	| "payloads"
	| "edge"
	| "fragmentation"
	| "schema"
	| "index"
	| "relational"
	| "constraints"
	| "savepoints"
	| "pragma"
	| "prepared"
	| "growth"
	| "readwrite"
	| "truncate"
	| "boundary-keys"
	| "shadow"
	| "actual-nul"
	| "nasty-script"
	| "nasty"
	| "kitchen-sink";

interface RunPhaseInput {
	seed: string;
	phase: number;
	iterations: number;
	mode?: WorkloadMode;
	maxPayloadBytes?: number;
	growthTargetBytes?: number;
	keySpace?: number;
}

interface ValidationSummary {
	totalEvents: number;
	activeRows: number;
	expectedRows: number;
	missingRows: number;
	extraRows: number;
	mismatchedRows: number;
	duplicateKeys: number;
	actualVersionSum: number;
	expectedVersionSum: number;
	actualPayloadChecksumSum: number;
	expectedPayloadChecksumSum: number;
	accountCount: number;
	accountBalanceSum: number;
	expectedAccountBalanceSum: number;
	accountBalanceMismatch: number;
	integrityCheck: string;
	quickCheck: string;
	edgeRows: number;
	edgeExpectedRows: number;
	edgeMismatches: number;
	indexRows: number;
	indexMismatches: number;
	relationalOrders: number;
	relationalMismatches: number;
	constraintAttempts: number;
	constraintLeaks: number;
	savepointRows: number;
	savepointMismatches: number;
	idempotentOps: number;
	idempotentMismatches: number;
	schemaObjects: number;
	schemaMissingObjects: number;
	probeRows: number;
	probeMismatches: number;
	preparedRows: number;
	preparedMismatches: number;
	shadowRows: number;
	shadowMismatches: number;
}

interface ItemMismatchDebugRow {
	itemKey: string;
	actualValue: string | null;
	expectedValue: string | null;
	actualVersion: number | null;
	expectedVersion: number | null;
	actualUpdateCount: number | null;
	expectedUpdateCount: number | null;
	actualPayloadChecksum: number | null;
	expectedPayloadChecksum: number | null;
	actualPayloadBytes: number | null;
	expectedPayloadBytes: number | null;
}

interface ItemEventDebugRow {
	seq: number;
	phase: number;
	localIndex: number;
	kind: string;
	present: number;
	value: string | null;
	version: number;
	updateCount: number;
	payloadChecksum: number;
	payloadBytes: number;
	applied: number;
}

interface PhaseResult {
	seed: string;
	phase: number;
	mode: WorkloadMode;
	iterations: number;
	ops: Record<string, number>;
	validation: ValidationSummary;
}

interface ItemRow {
	item_key: string;
	value: string;
	version: number;
	update_count: number;
	payload?: string;
	payload_checksum: number;
	payload_bytes: number;
}

const ACCOUNT_COUNT = 8;
const ACCOUNT_INITIAL_BALANCE = 100_000;
const DEFAULT_KEY_SPACE = 64;
const DEFAULT_MAX_PAYLOAD_BYTES = 8 * 1024;
const DEFAULT_GROWTH_TARGET_BYTES = 1024 * 1024;
const LARGE_WRITE_CHUNK_BYTES = 96 * 1024;
const PAGE_BOUNDARY_SIZES = [
	1,
	4095,
	4096,
	4097,
	8191,
	8192,
	8193,
	32768,
	65535,
	65536,
	98304,
	131072,
];

function hashSeed(input: string): number {
	let hash = 2166136261;
	for (let i = 0; i < input.length; i += 1) {
		hash ^= input.charCodeAt(i);
		hash = Math.imul(hash, 16777619);
	}
	return hash >>> 0;
}

function makeRng(seed: string): () => number {
	let state = hashSeed(seed) || 0x9e3779b9;
	return () => {
		state = (state + 0x6d2b79f5) >>> 0;
		let t = state;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

function intBetween(rng: () => number, min: number, max: number): number {
	return min + Math.floor(rng() * (max - min + 1));
}

function checksum(input: string): number {
	let hash = 2166136261;
	for (let i = 0; i < input.length; i += 1) {
		hash ^= input.charCodeAt(i);
		hash = Math.imul(hash, 16777619);
	}
	return hash >>> 0;
}

function payloadFor(
	seed: string,
	phase: number,
	index: number,
	bytes: number,
): string {
	const prefix = `${seed}:${phase}:${index}:`;
	if (bytes <= prefix.length) return prefix.slice(0, bytes);
	return prefix + "x".repeat(bytes - prefix.length);
}

async function queryOne<T>(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	sql: string,
	...args: unknown[]
): Promise<T | undefined> {
	const rows = await database.execute(sql, ...args);
	return rows[0] as T | undefined;
}

async function transaction<T>(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	fn: () => Promise<T>,
): Promise<T> {
	await database.execute("BEGIN");
	try {
		const result = await fn();
		await database.execute("COMMIT");
		return result;
	} catch (err) {
		await database.execute("ROLLBACK").catch(() => undefined);
		throw err;
	}
}

async function recordProbe(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	phase: number,
	scenario: string,
	name: string,
	expected: string | number,
	actual: string | number,
	mismatch: boolean,
): Promise<void> {
	await database.execute(
		`INSERT INTO fuzz_probe_results (
			phase, scenario, name, expected, actual, mismatch, created_at
		) VALUES (?, ?, ?, ?, ?, ?, ?)`,
		phase,
		scenario,
		name,
		String(expected),
		String(actual),
		mismatch ? 1 : 0,
		Date.now(),
	);
}

function firstColumn(row: unknown): unknown {
	if (!row || typeof row !== "object") return undefined;
	const values = Object.values(row as Record<string, unknown>);
	return values[0];
}

async function ensureAccounts(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
): Promise<void> {
	await database.execute("BEGIN");
	try {
		for (let i = 0; i < ACCOUNT_COUNT; i += 1) {
			await database.execute(
				"INSERT OR IGNORE INTO fuzz_accounts (id, balance) VALUES (?, ?)",
				`acct-${i}`,
				ACCOUNT_INITIAL_BALANCE,
			);
		}
		await database.execute("COMMIT");
	} catch (err) {
		await database.execute("ROLLBACK").catch(() => undefined);
		throw err;
	}
}

async function recordItemEvent(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	phase: number,
	localIndex: number,
	kind: string,
	itemKey: string,
	present: boolean,
	value: string | null,
	version: number,
	updateCount: number,
	payload: string,
	applied: boolean,
): Promise<void> {
	await database.execute(
		`INSERT INTO fuzz_item_events (
			phase, local_index, kind, item_key, present, value, version,
			update_count, payload_checksum, payload_bytes, applied, created_at
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		phase,
		localIndex,
		kind,
		itemKey,
		present ? 1 : 0,
		value,
		version,
		updateCount,
		checksum(payload),
		payload.length,
		applied ? 1 : 0,
		Date.now(),
	);
}

async function upsertLiveItem(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	row: ItemRow,
	payload: string,
): Promise<void> {
	await database.execute(
		`INSERT INTO fuzz_items (
			item_key, value, version, update_count, payload, payload_checksum,
			payload_bytes, updated_at
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
		ON CONFLICT(item_key) DO UPDATE SET
			value = excluded.value,
			version = excluded.version,
			update_count = excluded.update_count,
			payload = excluded.payload,
			payload_checksum = excluded.payload_checksum,
			payload_bytes = excluded.payload_bytes,
			updated_at = excluded.updated_at`,
		row.item_key,
		row.value,
		row.version,
		row.update_count,
		payload,
		row.payload_checksum,
		row.payload_bytes,
		Date.now(),
	);
}

async function applyItemOperation(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		seed: string;
		phase: number;
		localIndex: number;
		kind: "insert" | "update" | "delete" | "upsert";
		itemKey: string;
		payloadBytes: number;
	},
): Promise<void> {
	let current: ItemRow | undefined;
	try {
		current = await queryOne<ItemRow>(
			database,
			"SELECT item_key, value, version, update_count, payload, payload_checksum, payload_bytes FROM fuzz_items WHERE item_key = ?",
			opts.itemKey,
		);
	} catch (error) {
		throw new Error(
			`item operation select failed for kind ${opts.kind} key ${JSON.stringify(opts.itemKey)} payloadBytes ${opts.payloadBytes}`,
			{ cause: error },
		);
	}
	const payload = payloadFor(
		opts.seed,
		opts.phase,
		opts.localIndex,
		opts.payloadBytes,
	);
	const nextVersion = (current?.version ?? 0) + 1;
	const nextUpdateCount = (current?.update_count ?? 0) + 1;
	const nextValue = `${opts.kind}:${opts.phase}:${opts.localIndex}:${nextVersion}`;

	if (opts.kind === "delete") {
		try {
			await recordItemEvent(
				database,
				opts.phase,
				opts.localIndex,
				opts.kind,
				opts.itemKey,
				false,
				null,
				nextVersion,
				nextUpdateCount,
				"",
				current !== undefined,
			);
		} catch (error) {
			throw new Error(`item operation event insert failed for delete key ${JSON.stringify(opts.itemKey)}`, {
				cause: error,
			});
		}
		try {
			await database.execute("DELETE FROM fuzz_items WHERE item_key = ?", opts.itemKey);
		} catch (error) {
			throw new Error(`item operation delete failed for key ${JSON.stringify(opts.itemKey)}`, {
				cause: error,
			});
		}
		return;
	}

	if (opts.kind === "insert" && current) {
		try {
			await recordItemEvent(
				database,
				opts.phase,
				opts.localIndex,
				opts.kind,
				opts.itemKey,
				true,
				current.value,
				current.version,
				current.update_count,
				current.payload ?? "",
				false,
			);
		} catch (error) {
			throw new Error(
				`item operation event insert failed for noop insert key ${JSON.stringify(opts.itemKey)}`,
				{ cause: error },
			);
		}
		return;
	}

	const row: ItemRow = {
		item_key: opts.itemKey,
		value: nextValue,
		version: nextVersion,
		update_count: nextUpdateCount,
		payload_checksum: checksum(payload),
		payload_bytes: payload.length,
	};

	try {
		await recordItemEvent(
			database,
			opts.phase,
			opts.localIndex,
			opts.kind,
			opts.itemKey,
			true,
			row.value,
			row.version,
			row.update_count,
			payload,
			true,
		);
	} catch (error) {
		throw new Error(
			`item operation event insert failed for kind ${opts.kind} key ${JSON.stringify(opts.itemKey)} payloadBytes ${opts.payloadBytes}`,
			{ cause: error },
		);
	}
	try {
		await upsertLiveItem(database, row, payload);
	} catch (error) {
		throw new Error(
			`item operation live-row upsert failed for kind ${opts.kind} key ${JSON.stringify(opts.itemKey)} payloadBytes ${opts.payloadBytes}`,
			{ cause: error },
		);
	}
}

async function applyHotUpdates(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		seed: string;
		phase: number;
		localIndex: number;
		itemKey: string;
		updates: number;
		payloadBytes: number;
	},
): Promise<void> {
	for (let i = 0; i < opts.updates; i += 1) {
		try {
			await applyItemOperation(database, {
				seed: opts.seed,
				phase: opts.phase,
				localIndex: opts.localIndex * 1000 + i,
				kind: "update",
				itemKey: opts.itemKey,
				payloadBytes: opts.payloadBytes,
			});
		} catch (error) {
			throw new Error(
				`hot update failed for ${opts.itemKey} at sub-update ${i + 1}/${opts.updates} with payloadBytes ${opts.payloadBytes}`,
				{ cause: error },
			);
		}
	}
}

async function applyTransfer(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		phase: number;
		localIndex: number;
		fromAccount: string;
		toAccount: string;
		amount: number;
	},
): Promise<void> {
	await transaction(database, async () => {
		const before = await queryOne<{ total: number }>(
			database,
			"SELECT COALESCE(SUM(balance), 0) AS total FROM fuzz_accounts",
		);
		await database.execute(
			"UPDATE fuzz_accounts SET balance = balance - ? WHERE id = ?",
			opts.amount,
			opts.fromAccount,
		);
		await database.execute(
			"UPDATE fuzz_accounts SET balance = balance + ? WHERE id = ?",
			opts.amount,
			opts.toAccount,
		);
		const after = await queryOne<{ total: number }>(
			database,
			"SELECT COALESCE(SUM(balance), 0) AS total FROM fuzz_accounts",
		);
		await database.execute(
			`INSERT INTO fuzz_transfer_events (
				phase, local_index, from_account, to_account, amount,
				balance_sum_before, balance_sum_after, created_at
			) VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
			opts.phase,
			opts.localIndex,
			opts.fromAccount,
			opts.toAccount,
			opts.amount,
			before?.total ?? 0,
			after?.total ?? 0,
			Date.now(),
		);
	});
}

async function applyEdgePayloads(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		seed: string;
		phase: number;
		maxPayloadBytes: number;
	},
): Promise<number> {
	const writeEdgePayload = async (
		id: string,
		kind: string,
		payload: string,
		sizeLabel: string,
	): Promise<void> => {
		const payloadChecksum = checksum(payload);
		const payloadBytes = payload.length;
		try {
			await database.execute("BEGIN");
		} catch (error) {
			throw new Error(`edge payload begin failed for ${sizeLabel}`, {
				cause: error,
			});
		}
		try {
			try {
				await database.execute(
					`INSERT INTO fuzz_edge_payloads (
						id, kind, payload, payload_checksum, payload_bytes, updated_at
					) VALUES (?, ?, ?, ?, ?, ?)
					ON CONFLICT(id) DO UPDATE SET
						kind = excluded.kind,
						payload = excluded.payload,
						payload_checksum = excluded.payload_checksum,
						payload_bytes = excluded.payload_bytes,
						updated_at = excluded.updated_at`,
					id,
					kind,
					payload,
					payloadChecksum,
					payloadBytes,
					Date.now(),
				);
			} catch (error) {
				throw new Error(`edge payload row upsert failed for ${sizeLabel}`, {
					cause: error,
				});
			}
			try {
				await database.execute(
					`INSERT INTO fuzz_edge_expectations (
						id, present, payload_checksum, payload_bytes
					) VALUES (?, 1, ?, ?)
					ON CONFLICT(id) DO UPDATE SET
						present = excluded.present,
						payload_checksum = excluded.payload_checksum,
						payload_bytes = excluded.payload_bytes`,
					id,
					payloadChecksum,
					payloadBytes,
				);
			} catch (error) {
				throw new Error(`edge payload expectation upsert failed for ${sizeLabel}`, {
					cause: error,
				});
			}
			try {
				await database.execute("COMMIT");
			} catch (error) {
				throw new Error(`edge payload commit failed for ${sizeLabel}`, {
					cause: error,
				});
			}
		} catch (error) {
			await database.execute("ROLLBACK").catch(() => undefined);
			throw error;
		}
	};

	const sizes = PAGE_BOUNDARY_SIZES.filter((size) => size <= opts.maxPayloadBytes);
	if (!sizes.includes(opts.maxPayloadBytes)) sizes.push(opts.maxPayloadBytes);

	let ops = 0;
	for (const size of sizes) {
		const id = `edge-${opts.phase}-${size}`;
		const payload = payloadFor(opts.seed, opts.phase, size, size);
		try {
			await writeEdgePayload(id, "boundary", payload, `size ${size}`);
		} catch (error) {
			throw new Error(`edge payload write failed for size ${size}`, {
				cause: error,
			});
		}
		ops += 1;
	}

	const unicodePayload = `escaped-nul:\\0 unicode:☃️ phase:${opts.phase} seed:${opts.seed}`;
	const unicodeId = `edge-${opts.phase}-unicode-nul`;
	try {
		await writeEdgePayload(
			unicodeId,
			"unicode-nul",
			unicodePayload,
			"unicode escaped-nul payload",
		);
	} catch (error) {
		throw new Error("edge payload write failed for unicode escaped-nul payload", {
			cause: error,
		});
	}

	return ops + 1;
}

async function applyActualNulPayload(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		seed: string;
		phase: number;
	},
): Promise<number> {
	const payload = `actual-nul:\0 phase:${opts.phase} seed:${opts.seed}`;
	const id = `actual-nul-${opts.phase}`;
	await transaction(database, async () => {
		await database.execute(
			`INSERT INTO fuzz_edge_payloads (
				id, kind, payload, payload_checksum, payload_bytes, updated_at
			) VALUES (?, 'actual-nul', ?, ?, ?, ?)
			ON CONFLICT(id) DO UPDATE SET
				payload = excluded.payload,
				payload_checksum = excluded.payload_checksum,
				payload_bytes = excluded.payload_bytes,
				updated_at = excluded.updated_at`,
			id,
			payload,
			checksum(payload),
			payload.length,
			Date.now(),
		);
		await database.execute(
			`INSERT INTO fuzz_edge_expectations (
				id, present, payload_checksum, payload_bytes
			) VALUES (?, 1, ?, ?)
			ON CONFLICT(id) DO UPDATE SET
				present = 1,
				payload_checksum = excluded.payload_checksum,
				payload_bytes = excluded.payload_bytes`,
			id,
			checksum(payload),
			payload.length,
		);
	});
	return 1;
}

async function applyFragmentationChurn(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		seed: string;
		phase: number;
		rng: () => number;
		iterations: number;
		maxPayloadBytes: number;
	},
): Promise<number> {
	const rows = Math.max(12, Math.floor(opts.iterations / 2));
	let ops = 0;

	for (let i = 0; i < rows; i += 1) {
		const size = intBetween(opts.rng, 32, Math.max(32, opts.maxPayloadBytes));
		const id = `frag-${opts.phase}-${i}`;
		const payload = payloadFor(opts.seed, opts.phase, 10_000 + i, size);
		try {
			await database.execute(
				`INSERT INTO fuzz_edge_payloads (
					id, kind, payload, payload_checksum, payload_bytes, updated_at
				) VALUES (?, 'fragment', ?, ?, ?, ?)
				ON CONFLICT(id) DO UPDATE SET
					payload = excluded.payload,
					payload_checksum = excluded.payload_checksum,
					payload_bytes = excluded.payload_bytes,
					updated_at = excluded.updated_at`,
				id,
				payload,
				checksum(payload),
				payload.length,
				Date.now(),
			);
		} catch (error) {
			throw new Error(`fragmentation payload upsert failed for ${id} at size ${size}`, {
				cause: error,
			});
		}
		try {
			await database.execute(
				`INSERT INTO fuzz_edge_expectations (
					id, present, payload_checksum, payload_bytes
				) VALUES (?, 1, ?, ?)
				ON CONFLICT(id) DO UPDATE SET
					present = 1,
					payload_checksum = excluded.payload_checksum,
					payload_bytes = excluded.payload_bytes`,
				id,
				checksum(payload),
				payload.length,
			);
		} catch (error) {
			throw new Error(`fragmentation expectation upsert failed for ${id} at size ${size}`, {
				cause: error,
			});
		}
		ops += 1;
	}

	for (let i = 0; i < rows; i += 3) {
		const id = `frag-${opts.phase}-${i}`;
		try {
			await database.execute("DELETE FROM fuzz_edge_payloads WHERE id = ?", id);
		} catch (error) {
			throw new Error(`fragmentation delete failed for ${id}`, {
				cause: error,
			});
		}
		try {
			await database.execute(
				`INSERT INTO fuzz_edge_expectations (
					id, present, payload_checksum, payload_bytes
				) VALUES (?, 0, 0, 0)
				ON CONFLICT(id) DO UPDATE SET
					present = 0,
					payload_checksum = 0,
					payload_bytes = 0`,
				id,
			);
		} catch (error) {
			throw new Error(`fragmentation tombstone expectation failed for ${id}`, {
				cause: error,
			});
		}
		ops += 1;
	}

	for (let i = 1; i < rows; i += 4) {
		const size = intBetween(opts.rng, 1, Math.max(1, opts.maxPayloadBytes));
		const id = `frag-${opts.phase}-${i}`;
		const payload = payloadFor(opts.seed, opts.phase, 20_000 + i, size);
		try {
			await database.execute(
				`UPDATE fuzz_edge_payloads
				SET payload = ?, payload_checksum = ?, payload_bytes = ?, updated_at = ?
				WHERE id = ?`,
				payload,
				checksum(payload),
				payload.length,
				Date.now(),
				id,
			);
		} catch (error) {
			throw new Error(`fragmentation payload rewrite failed for ${id} at size ${size}`, {
				cause: error,
			});
		}
		try {
			await database.execute(
				`UPDATE fuzz_edge_expectations
				SET payload_checksum = ?, payload_bytes = ?
				WHERE id = ? AND present = 1`,
				checksum(payload),
				payload.length,
				id,
			);
		} catch (error) {
			throw new Error(`fragmentation expectation rewrite failed for ${id} at size ${size}`, {
				cause: error,
			});
		}
		ops += 1;
	}

	if (opts.phase % 2 === 1) {
		try {
			await database.execute("VACUUM");
		} catch (error) {
			throw new Error(`fragmentation vacuum failed for phase ${opts.phase}`, {
				cause: error,
			});
		}
		ops += 1;
	}

	return ops;
}

async function applySchemaChurn(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	phase: number,
): Promise<number> {
	const table = `fuzz_schema_phase_${phase}`;
	const index = `idx_fuzz_schema_phase_${phase}_name`;
	const view = `view_fuzz_schema_phase_${phase}`;
	const dropIndex = `idx_fuzz_schema_drop_probe_${phase}`;

	await database.execute(
		`CREATE TABLE IF NOT EXISTS ${table} (
			id INTEGER PRIMARY KEY,
			name TEXT NOT NULL UNIQUE,
			value INTEGER NOT NULL DEFAULT 0,
			extra TEXT
		)`,
	);
	await database.execute(`CREATE INDEX IF NOT EXISTS ${index} ON ${table}(name, value)`);
	try {
		await database.execute(`ALTER TABLE ${table} ADD COLUMN altered_${phase} TEXT DEFAULT 'altered'`);
	} catch {
		const column = await queryOne<{ count: number }>(
			database,
			`SELECT COUNT(*) AS count FROM pragma_table_info('${table}') WHERE name = ?`,
			`altered_${phase}`,
		);
		if ((column?.count ?? 0) !== 1) throw new Error(`failed to add altered_${phase}`);
	}
	await database.execute(`CREATE VIEW IF NOT EXISTS ${view} AS SELECT id, name, value FROM ${table}`);
	await database.execute(
		`INSERT INTO ${table} (name, value, extra)
		VALUES (?, ?, ?)
		ON CONFLICT(name) DO UPDATE SET
			value = excluded.value,
			extra = excluded.extra`,
		`schema-${phase}`,
		phase,
		`extra-${phase}`,
	);
	await database.execute(
		`CREATE TABLE IF NOT EXISTS fuzz_without_rowid (
			id TEXT PRIMARY KEY,
			value INTEGER NOT NULL
		) WITHOUT ROWID`,
	);
	await database.execute(`
		CREATE TRIGGER IF NOT EXISTS trg_fuzz_edge_payload_update
		AFTER UPDATE ON fuzz_edge_payloads
		BEGIN
			INSERT INTO fuzz_trigger_audit (
				payload_id, old_checksum, new_checksum, created_at
			) VALUES (
				new.id, old.payload_checksum, new.payload_checksum, strftime('%s', 'now') * 1000
			);
		END
	`);
	await database.execute(
		`INSERT INTO fuzz_without_rowid (id, value)
		VALUES (?, ?)
		ON CONFLICT(id) DO UPDATE SET value = excluded.value`,
		`phase-${phase}`,
		phase,
	);

	for (const [name, type] of [
		[table, "table"],
		[index, "index"],
		[view, "view"],
		["trg_fuzz_edge_payload_update", "trigger"],
		["fuzz_without_rowid", "table"],
	] as const) {
		await database.execute(
			`INSERT INTO fuzz_schema_registry (name, type)
			VALUES (?, ?)
			ON CONFLICT(name) DO UPDATE SET type = excluded.type`,
			name,
			type,
		);
	}

	await database.execute("CREATE TEMP TABLE IF NOT EXISTS fuzz_temp_probe (id INTEGER PRIMARY KEY, value TEXT)");
	await database.execute("INSERT INTO fuzz_temp_probe (value) VALUES (?)", `temp-${phase}`);
	await database.execute("DROP TABLE fuzz_temp_probe");
	await database.execute(`CREATE INDEX IF NOT EXISTS ${dropIndex} ON fuzz_schema_registry(type)`);
	await database.execute(`DROP INDEX IF EXISTS ${dropIndex}`);
	const dropped = await queryOne<{ count: number }>(
		database,
		"SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'index' AND name = ?",
		dropIndex,
	);
	await recordProbe(
		database,
		phase,
		"schema",
		"drop-index",
		0,
		dropped?.count ?? -1,
		(dropped?.count ?? -1) !== 0,
	);

	return 13;
}

async function applyIndexProbe(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		seed: string;
		phase: number;
		rng: () => number;
		iterations: number;
	},
): Promise<number> {
	const rows = Math.max(20, opts.iterations);
	await transaction(database, async () => {
		for (let i = 0; i < rows; i += 1) {
			const tenant = `tenant-${intBetween(opts.rng, 0, 5)}`;
			const bucket = intBetween(opts.rng, 0, 12);
			const score = intBetween(opts.rng, -500, 500);
			const label = `${opts.seed}:${opts.phase}:${i}`;
			await database.execute(
				`INSERT INTO fuzz_indexed (tenant, bucket, score, label, payload)
				VALUES (?, ?, ?, ?, ?)`,
				tenant,
				bucket,
				score,
				label,
				payloadFor(opts.seed, opts.phase, 30_000 + i, intBetween(opts.rng, 8, 256)),
			);
		}
	});
	return rows;
}

async function applyPreparedChurn(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		seed: string;
		phase: number;
		iterations: number;
		maxPayloadBytes: number;
	},
): Promise<number> {
	const rows = Math.max(32, opts.iterations);
	for (let i = 0; i < rows; i += 1) {
		const id = `prep-${opts.phase}-${i}`;
		const payload = payloadFor(
			opts.seed,
			opts.phase,
			70_000 + i,
			Math.min(opts.maxPayloadBytes, 64 + (i % 257)),
		);
		await database.execute(
			`INSERT INTO fuzz_prepared_churn (id, value, payload, payload_checksum)
			VALUES (?, ?, ?, ?)
			ON CONFLICT(id) DO UPDATE SET
				value = excluded.value,
				payload = excluded.payload,
				payload_checksum = excluded.payload_checksum
			/* unique-prepared-${opts.phase}-${i} */`,
			id,
			i,
			payload,
			checksum(payload),
		);
		await database.execute(
			`INSERT INTO fuzz_prepared_expectations (id, value, payload_checksum)
			VALUES (?, ?, ?)
			ON CONFLICT(id) DO UPDATE SET
				value = excluded.value,
				payload_checksum = excluded.payload_checksum`,
			id,
			i,
			checksum(payload),
		);
	}

	const repeatedId = `prep-repeat-${opts.phase}`;
	await database.execute(
		`INSERT INTO fuzz_prepared_churn (id, value, payload, payload_checksum)
		VALUES (?, 0, '', 0)
		ON CONFLICT(id) DO UPDATE SET value = 0, payload = '', payload_checksum = 0`,
		repeatedId,
	);
	for (let i = 0; i < rows; i += 1) {
		const payload = payloadFor(opts.seed, opts.phase, 80_000 + i, Math.min(512, opts.maxPayloadBytes));
		await database.execute(
			`UPDATE fuzz_prepared_churn
			SET value = value + ?, payload = ?, payload_checksum = ?
			WHERE id = ?`,
			1,
			payload,
			checksum(payload),
			repeatedId,
		);
	}
	const finalPayload = payloadFor(opts.seed, opts.phase, 80_000 + rows - 1, Math.min(512, opts.maxPayloadBytes));
	await database.execute(
		`INSERT INTO fuzz_prepared_expectations (id, value, payload_checksum)
		VALUES (?, ?, ?)
		ON CONFLICT(id) DO UPDATE SET
			value = excluded.value,
			payload_checksum = excluded.payload_checksum`,
		repeatedId,
		rows,
		checksum(finalPayload),
	);

	return rows * 2 + 1;
}

async function applyReadWriteProbe(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		seed: string;
		phase: number;
		rng: () => number;
		iterations: number;
	},
): Promise<number> {
	if ((await queryOne<{ count: number }>(database, "SELECT COUNT(*) AS count FROM fuzz_indexed"))?.count === 0) {
		await applyIndexProbe(database, opts);
	}

	const read = database.execute(
		`SELECT
			COUNT(*) AS joined_rows,
			COALESCE(SUM(a.score + b.score), 0) AS score_sum
		FROM fuzz_indexed a
		JOIN fuzz_indexed b ON b.bucket = a.bucket
		WHERE a.tenant <= 'tenant-3'`,
	);
	const write = applyIndexProbe(database, {
		seed: opts.seed,
		phase: opts.phase,
		rng: opts.rng,
		iterations: Math.max(10, Math.floor(opts.iterations / 2)),
	});
	const [readRows, writeOps] = await Promise.all([read, write]);
	const row = readRows[0] as { joined_rows?: number; score_sum?: number } | undefined;
	const joinedRows = Number(row?.joined_rows ?? -1);
	const scoreSum = Number(row?.score_sum ?? Number.NaN);
	await recordProbe(
		database,
		opts.phase,
		"readwrite",
		"long-read-while-write",
		"nonnegative-finite",
		`${joinedRows}:${scoreSum}`,
		joinedRows < 0 || !Number.isFinite(scoreSum),
	);
	return writeOps + 1;
}

async function applyBoundaryKeys(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		seed: string;
		phase: number;
		maxPayloadBytes: number;
	},
): Promise<number> {
	const keys = [
		"",
		" ",
		`long-${"k".repeat(2048)}`,
		"slash/key",
		"comma,key",
		"percent%key",
		"CaseKey",
		"casekey",
	];
	let ops = 0;
	for (const [index, key] of keys.entries()) {
		try {
			await applyItemOperation(database, {
				seed: opts.seed,
				phase: opts.phase,
				localIndex: 90_000 + index,
				kind: "upsert",
				itemKey: key,
				payloadBytes: Math.min(opts.maxPayloadBytes, 128 + index),
			});
		} catch (error) {
			throw new Error(
				`boundary key write failed for literal key ${JSON.stringify(key)} at index ${index}`,
				{ cause: error },
			);
		}
		ops += 1;
	}
	for (let i = 0; i < 128; i += 1) {
		const itemKey = `seq-${opts.phase}-${i.toString().padStart(4, "0")}`;
		try {
			await applyItemOperation(database, {
				seed: opts.seed,
				phase: opts.phase,
				localIndex: 91_000 + i,
				kind: i % 4 === 0 ? "delete" : "upsert",
				itemKey,
				payloadBytes: Math.min(opts.maxPayloadBytes, 32 + (i % 97)),
			});
		} catch (error) {
			throw new Error(
				`boundary key write failed for sequential key ${JSON.stringify(itemKey)} at index ${i}`,
				{ cause: error },
			);
		}
		ops += 1;
	}
	await recordProbe(database, opts.phase, "boundary-keys", "keys-written", 136, ops, ops !== 136);
	return ops;
}

async function applyGrowthProbe(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		seed: string;
		phase: number;
		maxPayloadBytes: number;
		growthTargetBytes: number;
	},
): Promise<number> {
	const chunkBytes = Math.max(1, Math.min(LARGE_WRITE_CHUNK_BYTES, opts.maxPayloadBytes));
	const rows = Math.max(1, Math.ceil(opts.growthTargetBytes / chunkBytes));
	let written = 0;
	for (let i = 0; i < rows; i += 1) {
		const size = Math.min(chunkBytes, opts.growthTargetBytes - written);
		const id = `growth-${opts.phase}-${opts.growthTargetBytes}-${i}`;
		const payload = payloadFor(opts.seed, opts.phase, 100_000 + i, size);
		await database.execute(
			`INSERT INTO fuzz_edge_payloads (
				id, kind, payload, payload_checksum, payload_bytes, updated_at
			) VALUES (?, 'growth', ?, ?, ?, ?)
			ON CONFLICT(id) DO UPDATE SET
				payload = excluded.payload,
				payload_checksum = excluded.payload_checksum,
				payload_bytes = excluded.payload_bytes,
				updated_at = excluded.updated_at`,
			id,
			payload,
			checksum(payload),
			payload.length,
			Date.now(),
		);
		await database.execute(
			`INSERT INTO fuzz_edge_expectations (
				id, present, payload_checksum, payload_bytes
			) VALUES (?, 1, ?, ?)
			ON CONFLICT(id) DO UPDATE SET
				present = 1,
				payload_checksum = excluded.payload_checksum,
				payload_bytes = excluded.payload_bytes`,
			id,
			checksum(payload),
			payload.length,
		);
		written += size;
	}
	await recordProbe(
		database,
		opts.phase,
		"growth",
		"target-bytes-written",
		opts.growthTargetBytes,
		written,
		written !== opts.growthTargetBytes,
	);
	return rows;
}

async function applyTruncateRecreateProbe(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		seed: string;
		phase: number;
		maxPayloadBytes: number;
	},
): Promise<number> {
	const id = `truncate-${opts.phase}`;
	const largeSize = Math.max(1, Math.min(opts.maxPayloadBytes, 131072));
	const largePayload = payloadFor(opts.seed, opts.phase, 110_000, largeSize);
	const tinyPayload = payloadFor(opts.seed, opts.phase, 110_001, 1);
	const recreatedPayload = payloadFor(opts.seed, opts.phase, 110_002, Math.min(4096, largeSize));

	await database.execute(
		`INSERT INTO fuzz_edge_payloads (
			id, kind, payload, payload_checksum, payload_bytes, updated_at
		) VALUES (?, 'truncate', ?, ?, ?, ?)
		ON CONFLICT(id) DO UPDATE SET
			payload = excluded.payload,
			payload_checksum = excluded.payload_checksum,
			payload_bytes = excluded.payload_bytes,
			updated_at = excluded.updated_at`,
		id,
		largePayload,
		checksum(largePayload),
		largePayload.length,
		Date.now(),
	);
	await database.execute(
		"UPDATE fuzz_edge_payloads SET payload = ?, payload_checksum = ?, payload_bytes = ?, updated_at = ? WHERE id = ?",
		tinyPayload,
		checksum(tinyPayload),
		tinyPayload.length,
		Date.now(),
		id,
	);
	await database.execute("DELETE FROM fuzz_edge_payloads WHERE id = ?", id);
	await database.execute("VACUUM");
	await database.execute(
		`INSERT INTO fuzz_edge_payloads (
			id, kind, payload, payload_checksum, payload_bytes, updated_at
		) VALUES (?, 'truncate', ?, ?, ?, ?)`,
		id,
		recreatedPayload,
		checksum(recreatedPayload),
		recreatedPayload.length,
		Date.now(),
	);
	await database.execute(
		`INSERT INTO fuzz_edge_expectations (
			id, present, payload_checksum, payload_bytes
		) VALUES (?, 1, ?, ?)
		ON CONFLICT(id) DO UPDATE SET
			present = 1,
			payload_checksum = excluded.payload_checksum,
			payload_bytes = excluded.payload_bytes`,
		id,
		checksum(recreatedPayload),
		recreatedPayload.length,
	);
	return 5;
}

async function updateShadowChecksums(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	phase: number,
): Promise<number> {
	const item = await queryOne<{ rows: number; value: number }>(
		database,
		`SELECT COUNT(*) AS rows, COALESCE(SUM(payload_checksum + version + update_count), 0) AS value
		FROM fuzz_items`,
	);
	const edge = await queryOne<{ rows: number; value: number }>(
		database,
		`SELECT COUNT(*) AS rows, COALESCE(SUM(payload_checksum + payload_bytes), 0) AS value
		FROM fuzz_edge_payloads`,
	);
	await transaction(database, async () => {
		for (const [name, rows, value] of [
			["items", item?.rows ?? 0, item?.value ?? 0],
			["edge", edge?.rows ?? 0, edge?.value ?? 0],
		] as const) {
			await database.execute(
				`INSERT INTO fuzz_shadow_checksums (name, value, row_count)
				VALUES (?, ?, ?)
				ON CONFLICT(name) DO UPDATE SET
					value = excluded.value,
					row_count = excluded.row_count`,
				name,
				value,
				rows,
			);
		}
	});
	await recordProbe(database, phase, "shadow", "shadow-updated", 2, 2, false);
	return 2;
}

async function applyConstraintChaos(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	phase: number,
): Promise<number> {
	await database.execute("PRAGMA foreign_keys = ON");
	const validPrefix = `valid-${phase}`;
	const existingValidRows = await queryOne<{ count: number }>(
		database,
		"SELECT COUNT(*) AS count FROM fuzz_constraints WHERE id LIKE ?",
		`${validPrefix}-%`,
	);
	const runSeq = existingValidRows?.count ?? 0;
	const validId = `${validPrefix}-${runSeq}`;
	const uniqValue = `uniq-${phase}-${runSeq}`;
	const before = await queryOne<{ count: number }>(
		database,
		"SELECT COUNT(*) AS count FROM fuzz_constraints",
	);
	await database.execute(
		`INSERT INTO fuzz_constraints (id, must_not_null, qty, uniq)
		VALUES (?, ?, ?, ?)
		ON CONFLICT(id) DO UPDATE SET
			must_not_null = excluded.must_not_null,
			qty = excluded.qty,
			uniq = excluded.uniq`,
		validId,
		"ok",
		phase,
		uniqValue,
	);

	const attempts: Array<{
		name: string;
		sql: string;
		args: unknown[];
	}> = [
		{
			name: `not-null-${phase}-${runSeq}`,
			sql: "INSERT INTO fuzz_constraints (id, must_not_null, qty, uniq) VALUES (?, ?, ?, ?)",
			args: [`bad-null-${phase}-${runSeq}`, null, 1, `bad-null-${phase}-${runSeq}`],
		},
		{
			name: `check-${phase}-${runSeq}`,
			sql: "INSERT INTO fuzz_constraints (id, must_not_null, qty, uniq) VALUES (?, ?, ?, ?)",
			args: [`bad-check-${phase}-${runSeq}`, "ok", -1, `bad-check-${phase}-${runSeq}`],
		},
		{
			name: `unique-${phase}-${runSeq}`,
			sql: "INSERT INTO fuzz_constraints (id, must_not_null, qty, uniq) VALUES (?, ?, ?, ?)",
			args: [`bad-unique-${phase}-${runSeq}`, "ok", 1, uniqValue],
		},
	];

	for (const attempt of attempts) {
		const attemptBefore = await queryOne<{ count: number }>(
			database,
			"SELECT COUNT(*) AS count FROM fuzz_constraints",
		);
		let failed = false;
		try {
			await database.execute(attempt.sql, ...attempt.args);
		} catch {
			failed = true;
		}
		const attemptAfter = await queryOne<{ count: number }>(
			database,
			"SELECT COUNT(*) AS count FROM fuzz_constraints",
		);
		await database.execute(
			`INSERT INTO fuzz_constraint_attempts (
				name, expected_failed, actually_failed, before_count, after_count
			) VALUES (?, 1, ?, ?, ?)`,
			attempt.name,
			failed ? 1 : 0,
			attemptBefore?.count ?? 0,
			attemptAfter?.count ?? 0,
		);
	}

	const after = await queryOne<{ count: number }>(
		database,
		"SELECT COUNT(*) AS count FROM fuzz_constraints",
	);
	if ((after?.count ?? 0) !== (before?.count ?? 0) + 1) {
		await database.execute(
			`INSERT INTO fuzz_constraint_attempts (
				name, expected_failed, actually_failed, before_count, after_count
			) VALUES (?, 0, 0, ?, ?)`,
			`valid-count-${phase}-${runSeq}`,
			before?.count ?? 0,
			after?.count ?? 0,
		);
	}

	const parentId = `fk-parent-${phase}-${runSeq}`;
	const childId = `fk-child-${phase}-${runSeq}`;
	await database.execute(
		"INSERT INTO fuzz_fk_parent (id) VALUES (?) ON CONFLICT(id) DO NOTHING",
		parentId,
	);
	await database.execute(
		`INSERT INTO fuzz_fk_child (id, parent_id)
		VALUES (?, ?)
		ON CONFLICT(id) DO UPDATE SET parent_id = excluded.parent_id`,
		childId,
		parentId,
	);
	const childBeforeDelete = await queryOne<{ count: number }>(
		database,
		"SELECT COUNT(*) AS count FROM fuzz_fk_child WHERE parent_id = ?",
		parentId,
	);
	await database.execute("DELETE FROM fuzz_fk_parent WHERE id = ?", parentId);
	const childAfterDelete = await queryOne<{ count: number }>(
		database,
		"SELECT COUNT(*) AS count FROM fuzz_fk_child WHERE parent_id = ?",
		parentId,
	);
	await recordProbe(
		database,
		phase,
		"constraints",
		"fk-cascade-delete",
		0,
		childAfterDelete?.count ?? -1,
		(childBeforeDelete?.count ?? 0) !== 1 || (childAfterDelete?.count ?? -1) !== 0,
	);

	const fkBefore = await queryOne<{ count: number }>(
		database,
		"SELECT COUNT(*) AS count FROM fuzz_fk_child",
	);
	let fkFailed = false;
	try {
		await database.execute(
			"INSERT INTO fuzz_fk_child (id, parent_id) VALUES (?, ?)",
			`fk-orphan-${phase}-${runSeq}`,
			`missing-parent-${phase}-${runSeq}`,
		);
	} catch {
		fkFailed = true;
	}
	const fkAfter = await queryOne<{ count: number }>(
		database,
		"SELECT COUNT(*) AS count FROM fuzz_fk_child",
	);
	await recordProbe(
		database,
		phase,
		"constraints",
		"fk-failure-isolation",
		`${fkBefore?.count ?? 0}:failed`,
		`${fkAfter?.count ?? 0}:${fkFailed ? "failed" : "inserted"}`,
		!fkFailed || (fkAfter?.count ?? 0) !== (fkBefore?.count ?? 0),
	);

	return attempts.length + 3;
}

async function applyPragmaProbe(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	phase: number,
): Promise<number> {
	let ops = 0;
	for (const [name, setupSql, checkSql, expected] of [
		["journal_mode", "PRAGMA journal_mode = DELETE", "PRAGMA journal_mode", "nonempty"],
		["synchronous", "PRAGMA synchronous = NORMAL", "PRAGMA synchronous", "nonempty"],
		["cache_size", "PRAGMA cache_size = -2000", "PRAGMA cache_size", "-2000"],
		["foreign_keys", "PRAGMA foreign_keys = ON", "PRAGMA foreign_keys", "1"],
		["auto_vacuum", "PRAGMA auto_vacuum", "PRAGMA auto_vacuum", "nonempty"],
	] as const) {
		try {
			await database.execute(setupSql);
			const rows = await database.execute(checkSql);
			const actual = String(firstColumn(rows[0]) ?? "");
			await recordProbe(
				database,
				phase,
				"pragma",
				name,
				expected,
				actual,
				expected === "nonempty" ? actual.length === 0 : actual !== expected,
			);
		} catch (err) {
			await recordProbe(
				database,
				phase,
				"pragma",
				name,
				expected,
				err instanceof Error ? err.message : "unknown error",
				true,
			);
		}
		ops += 1;
	}
	return ops;
}

async function applySavepointScenario(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	phase: number,
): Promise<number> {
	const keepId = `save-keep-${phase}`;
	const rolledBackId = `save-rolled-back-${phase}`;

	await database.execute("BEGIN");
	try {
		await database.execute(
			"INSERT INTO fuzz_savepoints (id, value) VALUES (?, ?) ON CONFLICT(id) DO UPDATE SET value = excluded.value",
			keepId,
			phase,
		);
		await database.execute("SAVEPOINT sp_rollback_probe");
		await database.execute(
			"INSERT INTO fuzz_savepoints (id, value) VALUES (?, ?) ON CONFLICT(id) DO UPDATE SET value = excluded.value",
			rolledBackId,
			999_000 + phase,
		);
		await database.execute(
			"UPDATE fuzz_savepoints SET value = value + 1000 WHERE id = ?",
			keepId,
		);
		await database.execute("ROLLBACK TO sp_rollback_probe");
		await database.execute("RELEASE sp_rollback_probe");
		await database.execute("COMMIT");
	} catch (err) {
		await database.execute("ROLLBACK").catch(() => undefined);
		throw err;
	}

	await database.execute(
		`INSERT INTO fuzz_savepoint_expectations (id, present, value)
		VALUES (?, 1, ?)
		ON CONFLICT(id) DO UPDATE SET present = 1, value = excluded.value`,
		keepId,
		phase,
	);
	await database.execute(
		`INSERT INTO fuzz_savepoint_expectations (id, present, value)
		VALUES (?, 0, 0)
		ON CONFLICT(id) DO UPDATE SET present = 0, value = 0`,
		rolledBackId,
	);

	return 5;
}

async function applyIdempotentReplay(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	phase: number,
): Promise<number> {
	const targetId = `idem-target-${phase % 3}`;
	await database.execute(
		"INSERT OR IGNORE INTO fuzz_idempotent_targets (id, value) VALUES (?, 0)",
		targetId,
	);

	for (let i = 0; i < 8; i += 1) {
		const opId = `idem-${phase}-${i}`;
		const amount = phase + i + 1;
		for (let attempt = 0; attempt < 3; attempt += 1) {
			await transaction(database, async () => {
				const existing = await queryOne<{ op_id: string }>(
					database,
					"SELECT op_id FROM fuzz_idempotent_ops WHERE op_id = ?",
					opId,
				);
				if (!existing) {
					await database.execute(
						"INSERT INTO fuzz_idempotent_ops (op_id, target_id, amount) VALUES (?, ?, ?)",
						opId,
						targetId,
						amount,
					);
					await database.execute(
						"UPDATE fuzz_idempotent_targets SET value = value + ? WHERE id = ?",
						amount,
						targetId,
					);
				}
			});
		}
	}

	return 24;
}

async function ensureRelationalSeed(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
): Promise<void> {
	await transaction(database, async () => {
		for (let i = 0; i < 8; i += 1) {
			await database.execute(
				"INSERT OR IGNORE INTO fuzz_rel_users (id, name) VALUES (?, ?)",
				`user-${i}`,
				`User ${i}`,
			);
		}
		for (let i = 0; i < 12; i += 1) {
			const productId = `product-${i}`;
			const initialQty = 10_000;
			await database.execute(
				"INSERT OR IGNORE INTO fuzz_rel_products (id, price) VALUES (?, ?)",
				productId,
				(i + 1) * 7,
			);
			await database.execute(
				`INSERT OR IGNORE INTO fuzz_inventory (
					product_id, initial_qty, sold_qty, stock_qty
				) VALUES (?, ?, 0, ?)`,
				productId,
				initialQty,
				initialQty,
			);
		}
	});
}

async function applyRelationalOrder(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		phase: number;
		localIndex: number;
		rng: () => number;
	},
): Promise<number> {
	await ensureRelationalSeed(database);
	const orderPrefix = `order-${opts.phase}-${opts.localIndex}`;
	const existingOrders = await queryOne<{ count: number }>(
		database,
		"SELECT COUNT(*) AS count FROM fuzz_orders WHERE id LIKE ?",
		`${orderPrefix}-%`,
	);
	const orderId = `${orderPrefix}-${existingOrders?.count ?? 0}`;
	const userId = `user-${intBetween(opts.rng, 0, 7)}`;
	const itemCount = intBetween(opts.rng, 1, 4);
	let total = 0;

	await transaction(database, async () => {
		await database.execute(
			"INSERT INTO fuzz_orders (id, user_id, total, status) VALUES (?, ?, 0, 'open')",
			orderId,
			userId,
		);
		for (let i = 0; i < itemCount; i += 1) {
			const productId = `product-${intBetween(opts.rng, 0, 11)}`;
			const product = await queryOne<{ price: number }>(
				database,
				"SELECT price FROM fuzz_rel_products WHERE id = ?",
				productId,
			);
			const quantity = intBetween(opts.rng, 1, 5);
			const price = product?.price ?? 0;
			total += price * quantity;
			await database.execute(
				`INSERT INTO fuzz_order_items (
					order_id, product_id, quantity, price
				) VALUES (?, ?, ?, ?)`,
				orderId,
				productId,
				quantity,
				price,
			);
			await database.execute(
				`UPDATE fuzz_inventory
				SET sold_qty = sold_qty + ?, stock_qty = stock_qty - ?
				WHERE product_id = ?`,
				quantity,
				quantity,
				productId,
			);
		}
		await database.execute(
			"UPDATE fuzz_orders SET total = ?, status = 'paid' WHERE id = ?",
			total,
			orderId,
		);
		await database.execute(
			"INSERT INTO fuzz_payments (order_id, amount, status) VALUES (?, ?, 'captured')",
			orderId,
			total,
		);
	});

	return itemCount + 4;
}

async function applyRollbackProbe(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	phase: number,
	rowCount = 20,
): Promise<number> {
	const before = await queryOne<{ count: number }>(
		database,
		"SELECT COUNT(*) AS count FROM fuzz_items WHERE item_key LIKE ?",
		`rollback-${phase}-%`,
	);
	await database.execute("BEGIN");
	try {
		for (let i = 0; i < rowCount; i += 1) {
			await database.execute(
				`INSERT INTO fuzz_items (
					item_key, value, version, update_count, payload, payload_checksum,
					payload_bytes, updated_at
				) VALUES (?, ?, 1, 1, ?, ?, ?, ?)`,
				`rollback-${phase}-${i}`,
				"should-not-survive",
				"rollback-payload",
				checksum("rollback-payload"),
				"rollback-payload".length,
				Date.now(),
			);
		}
		throw new Error("intentional rollback probe");
	} catch {
		await database.execute("ROLLBACK");
	}
	const after = await queryOne<{ count: number }>(
		database,
		"SELECT COUNT(*) AS count FROM fuzz_items WHERE item_key LIKE ?",
		`rollback-${phase}-%`,
	);
	await database.execute(
		`INSERT INTO fuzz_constraint_attempts (
			name, expected_failed, actually_failed, before_count, after_count
		) VALUES (?, 1, 1, ?, ?)`,
		`rollback-probe-${phase}`,
		before?.count ?? 0,
		after?.count ?? 0,
	);
	return rowCount;
}

async function applyNastyScript(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		seed: string;
		phase: number;
		maxPayloadBytes: number;
		intense?: boolean;
	},
): Promise<number> {
	const growKey = `nasty-grow-${opts.phase}`;
	let ops = 0;
	const growMax = Math.min(opts.maxPayloadBytes, 131072);
	const growSizes = PAGE_BOUNDARY_SIZES.filter((size) => size <= growMax);
	if (!growSizes.includes(1)) growSizes.unshift(1);
	if (!growSizes.includes(growMax)) growSizes.push(growMax);
	for (const size of growSizes) {
		await applyItemOperation(database, {
			seed: opts.seed,
			phase: opts.phase,
			localIndex: 50_000 + size,
			kind: "upsert",
			itemKey: growKey,
			payloadBytes: Math.min(size, opts.maxPayloadBytes),
		});
		ops += 1;
	}

	const hotUpdates = opts.intense ? 10_000 : 250;
	await applyHotUpdates(database, {
		seed: opts.seed,
		phase: opts.phase,
		localIndex: 60_000,
		itemKey: `nasty-hot-${opts.phase}`,
		updates: hotUpdates,
		payloadBytes: Math.min(1024, opts.maxPayloadBytes),
	});
	ops += hotUpdates;

	if (opts.intense) {
		await database.execute("CREATE INDEX IF NOT EXISTS idx_nasty_heavy_write ON fuzz_items(value, version)");
		for (let i = 0; i < 10_000; i += 1) {
			await applyItemOperation(database, {
				seed: opts.seed,
				phase: opts.phase,
				localIndex: 120_000 + i,
				kind: "upsert",
				itemKey: `nasty-bulk-${opts.phase}-${i}`,
				payloadBytes: Math.min(256, opts.maxPayloadBytes),
			});
			ops += 1;
		}
		for (let i = 0; i < 10_000; i += 2) {
			await applyItemOperation(database, {
				seed: opts.seed,
				phase: opts.phase,
				localIndex: 140_000 + i,
				kind: "delete",
				itemKey: `nasty-bulk-${opts.phase}-${i}`,
				payloadBytes: 1,
			});
			ops += 1;
		}
		await database.execute("DROP INDEX IF EXISTS idx_nasty_heavy_write");
	}

	const rollbackRows = opts.intense ? 1000 : 20;
	await applyRollbackProbe(database, opts.phase, rollbackRows);
	ops += rollbackRows;

	return ops;
}

async function applyDeterministicNastyScript(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		seed: string;
		phase: number;
		maxPayloadBytes: number;
	},
): Promise<number> {
	let ops = 0;
	const growId = `nasty-script-grow-${opts.phase}`;
	const maxGrowBytes = Math.min(opts.maxPayloadBytes, 131072);
	let finalGrowPayload = "";
	await transaction(database, async () => {
		for (let i = 0; i < 256; i += 1) {
			const size = Math.max(1, Math.floor(1 + ((maxGrowBytes - 1) * i) / 255));
			const payload = payloadFor(opts.seed, opts.phase, 160_000 + i, size);
			finalGrowPayload = payload;
			await database.execute(
				`INSERT INTO fuzz_edge_payloads (
					id, kind, payload, payload_checksum, payload_bytes, updated_at
				) VALUES (?, 'nasty-grow', ?, ?, ?, ?)
				ON CONFLICT(id) DO UPDATE SET
					payload = excluded.payload,
					payload_checksum = excluded.payload_checksum,
					payload_bytes = excluded.payload_bytes,
					updated_at = excluded.updated_at`,
				growId,
				payload,
				checksum(payload),
				payload.length,
				Date.now(),
			);
			ops += 1;
		}
		await database.execute(
			`INSERT INTO fuzz_edge_expectations (
				id, present, payload_checksum, payload_bytes
			) VALUES (?, 1, ?, ?)
			ON CONFLICT(id) DO UPDATE SET
				present = 1,
				payload_checksum = excluded.payload_checksum,
				payload_bytes = excluded.payload_bytes`,
			growId,
			checksum(finalGrowPayload),
			finalGrowPayload.length,
		);
	});

	const counterId = `nasty-counter-${opts.phase}`;
	await database.execute(
		"INSERT INTO fuzz_nasty_counter (id, value) VALUES (?, 0) ON CONFLICT(id) DO UPDATE SET value = 0",
		counterId,
	);
	await transaction(database, async () => {
		for (let i = 0; i < 10_000; i += 1) {
			await database.execute(
				"UPDATE fuzz_nasty_counter SET value = value + 1 WHERE id = ?",
				counterId,
			);
			ops += 1;
		}
	});
	const counter = await queryOne<{ value: number }>(
		database,
		"SELECT value FROM fuzz_nasty_counter WHERE id = ?",
		counterId,
	);
	await recordProbe(
		database,
		opts.phase,
		"nasty-script",
		"same-row-10k-updates",
		10_000,
		counter?.value ?? -1,
		(counter?.value ?? -1) !== 10_000,
	);

	const groupId = `nasty-bulk-${opts.phase}`;
	await database.execute("CREATE INDEX IF NOT EXISTS idx_fuzz_nasty_rows_group_n ON fuzz_nasty_rows(group_id, n)");
	await transaction(database, async () => {
		for (let i = 0; i < 10_000; i += 1) {
			await database.execute(
				`INSERT INTO fuzz_nasty_rows (group_id, n, payload)
				VALUES (?, ?, ?)
				ON CONFLICT(group_id, n) DO UPDATE SET payload = excluded.payload`,
				groupId,
				i,
				payloadFor(opts.seed, opts.phase, 170_000 + i, 64),
			);
			ops += 1;
		}
		await database.execute("DELETE FROM fuzz_nasty_rows WHERE group_id = ? AND n % 2 = 0", groupId);
		ops += 1;
	});
	await database.execute("DROP INDEX IF EXISTS idx_fuzz_nasty_rows_group_n");
	const remaining = await queryOne<{ count: number }>(
		database,
		"SELECT COUNT(*) AS count FROM fuzz_nasty_rows WHERE group_id = ?",
		groupId,
	);
	await recordProbe(
		database,
		opts.phase,
		"nasty-script",
		"insert-10k-delete-every-other",
		5000,
		remaining?.count ?? -1,
		(remaining?.count ?? -1) !== 5000,
	);
	const indexLeft = await queryOne<{ count: number }>(
		database,
		"SELECT COUNT(*) AS count FROM sqlite_master WHERE type = 'index' AND name = 'idx_fuzz_nasty_rows_group_n'",
	);
	await recordProbe(
		database,
		opts.phase,
		"nasty-script",
		"create-drop-index-around-heavy-writes",
		0,
		indexLeft?.count ?? -1,
		(indexLeft?.count ?? -1) !== 0,
	);

	const rollbackGroupId = `nasty-rollback-${opts.phase}`;
	const beforeRollback = await queryOne<{ count: number }>(
		database,
		"SELECT COUNT(*) AS count FROM fuzz_nasty_rows WHERE group_id = ?",
		rollbackGroupId,
	);
	await database.execute("BEGIN");
	try {
		for (let i = 0; i < 1000; i += 1) {
			await database.execute(
				"INSERT INTO fuzz_nasty_rows (group_id, n, payload) VALUES (?, ?, ?)",
				rollbackGroupId,
				i,
				"rollback",
			);
			ops += 1;
		}
		await database.execute("ROLLBACK");
	} catch (err) {
		await database.execute("ROLLBACK").catch(() => undefined);
		throw err;
	}
	const afterRollback = await queryOne<{ count: number }>(
		database,
		"SELECT COUNT(*) AS count FROM fuzz_nasty_rows WHERE group_id = ?",
		rollbackGroupId,
	);
	await recordProbe(
		database,
		opts.phase,
		"nasty-script",
		"rollback-1k-inserts",
		beforeRollback?.count ?? 0,
		afterRollback?.count ?? -1,
		(afterRollback?.count ?? -1) !== (beforeRollback?.count ?? 0),
	);

	return ops;
}

function shouldRunDeepScenario(mode: WorkloadMode, scenario: WorkloadMode): boolean {
	return mode === scenario || mode === "kitchen-sink" || mode === "nasty";
}

async function applyDeepScenarios(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	opts: {
		seed: string;
		phase: number;
		mode: WorkloadMode;
		iterations: number;
		rng: () => number;
		maxPayloadBytes: number;
		growthTargetBytes: number;
		ops: Record<string, number>;
	},
): Promise<void> {
	const runScenario = async <T>(name: string, fn: () => Promise<T>): Promise<T> => {
		try {
			return await fn();
		} catch (error) {
			const detail =
				error instanceof Error ? error.message : typeof error === "string" ? error : String(error);
			throw new Error(
				`deep scenario ${name} failed in mode ${opts.mode} during phase ${opts.phase}: ${detail}`,
				{ cause: error },
			);
		}
	};

	if (shouldRunDeepScenario(opts.mode, "edge") || opts.mode === "payloads") {
		opts.ops.edgePayload = (opts.ops.edgePayload ?? 0) +
			await runScenario("edge", () => applyEdgePayloads(database, opts));
	}
	if (opts.mode === "actual-nul") {
		opts.ops.actualNul = (opts.ops.actualNul ?? 0) +
			await runScenario("actual-nul", () => applyActualNulPayload(database, opts));
	}
	if (shouldRunDeepScenario(opts.mode, "fragmentation")) {
		opts.ops.fragmentation = (opts.ops.fragmentation ?? 0) +
			await runScenario("fragmentation", () => applyFragmentationChurn(database, opts));
	}
	if (shouldRunDeepScenario(opts.mode, "schema")) {
		opts.ops.schema = (opts.ops.schema ?? 0) +
			await runScenario("schema", () => applySchemaChurn(database, opts.phase));
	}
	if (shouldRunDeepScenario(opts.mode, "index")) {
		opts.ops.index = (opts.ops.index ?? 0) +
			await runScenario("index", () => applyIndexProbe(database, opts));
	}
	if (shouldRunDeepScenario(opts.mode, "constraints")) {
		opts.ops.constraints = (opts.ops.constraints ?? 0) +
			await runScenario("constraints", () => applyConstraintChaos(database, opts.phase));
	}
	if (shouldRunDeepScenario(opts.mode, "savepoints")) {
		opts.ops.savepoints = (opts.ops.savepoints ?? 0) +
			await runScenario("savepoints", () => applySavepointScenario(database, opts.phase));
	}
	if (shouldRunDeepScenario(opts.mode, "pragma")) {
		opts.ops.pragma = (opts.ops.pragma ?? 0) +
			await runScenario("pragma", () => applyPragmaProbe(database, opts.phase));
	}
	if (shouldRunDeepScenario(opts.mode, "prepared")) {
		opts.ops.prepared = (opts.ops.prepared ?? 0) +
			await runScenario("prepared", () => applyPreparedChurn(database, opts));
	}
	if (shouldRunDeepScenario(opts.mode, "growth")) {
		opts.ops.growth = (opts.ops.growth ?? 0) +
			await runScenario("growth", () => applyGrowthProbe(database, opts));
	}
	if (shouldRunDeepScenario(opts.mode, "readwrite")) {
		opts.ops.readwrite = (opts.ops.readwrite ?? 0) +
			await runScenario("readwrite", () => applyReadWriteProbe(database, opts));
	}
	if (shouldRunDeepScenario(opts.mode, "truncate")) {
		opts.ops.truncate = (opts.ops.truncate ?? 0) +
			await runScenario("truncate", () => applyTruncateRecreateProbe(database, opts));
	}
	if (shouldRunDeepScenario(opts.mode, "boundary-keys")) {
		opts.ops.boundaryKeys = (opts.ops.boundaryKeys ?? 0) +
			await runScenario("boundary-keys", () => applyBoundaryKeys(database, opts));
	}
	if (shouldRunDeepScenario(opts.mode, "relational")) {
		const orders = Math.max(1, Math.floor(opts.iterations / 20));
		for (let i = 0; i < orders; i += 1) {
			opts.ops.relational = (opts.ops.relational ?? 0) +
				await runScenario("relational", () =>
					applyRelationalOrder(database, {
						phase: opts.phase,
						localIndex: i,
						rng: opts.rng,
					}),
				);
		}
	}
	if (opts.mode === "kitchen-sink" || opts.mode === "nasty") {
		opts.ops.idempotent = (opts.ops.idempotent ?? 0) +
			await runScenario("idempotent", () => applyIdempotentReplay(database, opts.phase));
		opts.ops.nasty = (opts.ops.nasty ?? 0) +
			await runScenario("nasty", () =>
				applyNastyScript(database, { ...opts, intense: opts.mode === "nasty" }),
			);
	}
	if (opts.mode === "nasty-script") {
		opts.ops.nasty = (opts.ops.nasty ?? 0) +
			await runScenario("nasty-script", () => applyDeterministicNastyScript(database, opts));
	}
	if (shouldRunDeepScenario(opts.mode, "shadow")) {
		opts.ops.shadow = (opts.ops.shadow ?? 0) +
			await runScenario("shadow", () => updateShadowChecksums(database, opts.phase));
	}
}

function chooseKind(
	mode: WorkloadMode,
	rng: () => number,
): "insert" | "update" | "delete" | "upsert" | "hot" | "transfer" {
	const roll = rng();
	if (mode === "transactions") {
		if (roll < 0.55) return "transfer";
		if (roll < 0.75) return "upsert";
		if (roll < 0.9) return "update";
		return "delete";
	}
	if (mode === "hot") {
		if (roll < 0.6) return "hot";
		if (roll < 0.75) return "upsert";
		if (roll < 0.9) return "update";
		return "delete";
	}
	if (mode === "payloads") {
		if (roll < 0.4) return "upsert";
		if (roll < 0.7) return "insert";
		if (roll < 0.9) return "update";
		return "delete";
	}
	if (roll < 0.2) return "insert";
	if (roll < 0.45) return "update";
	if (roll < 0.65) return "delete";
	if (roll < 0.85) return "upsert";
	if (roll < 0.95) return "hot";
	return "transfer";
}

async function validate(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
): Promise<ValidationSummary> {
	const integrity = await queryOne<{ integrity_check: string }>(
		database,
		"PRAGMA integrity_check",
	);
	const quick = await queryOne<{ quick_check: string }>(
		database,
		"PRAGMA quick_check",
	);
	const totals = await queryOne<{
		total_events: number;
		active_rows: number;
		expected_rows: number;
		actual_version_sum: number;
		expected_version_sum: number;
		actual_payload_checksum_sum: number;
		expected_payload_checksum_sum: number;
	}>(
		database,
		`WITH latest AS (
			SELECT e.*
			FROM fuzz_item_events e
			JOIN (
				SELECT item_key, MAX(seq) AS seq
				FROM fuzz_item_events
				GROUP BY item_key
			) m ON m.item_key = e.item_key AND m.seq = e.seq
		)
		SELECT
			(SELECT COUNT(*) FROM fuzz_item_events) AS total_events,
			(SELECT COUNT(*) FROM fuzz_items) AS active_rows,
			(SELECT COUNT(*) FROM latest WHERE present = 1) AS expected_rows,
			(SELECT COALESCE(SUM(version), 0) FROM fuzz_items) AS actual_version_sum,
			(SELECT COALESCE(SUM(version), 0) FROM latest WHERE present = 1) AS expected_version_sum,
			(SELECT COALESCE(SUM(payload_checksum), 0) FROM fuzz_items) AS actual_payload_checksum_sum,
			(SELECT COALESCE(SUM(payload_checksum), 0) FROM latest WHERE present = 1) AS expected_payload_checksum_sum`,
	);
	const mismatches = await queryOne<{
		missing_rows: number;
		extra_rows: number;
		mismatched_rows: number;
		duplicate_keys: number;
	}>(
		database,
		`WITH latest AS (
			SELECT e.*
			FROM fuzz_item_events e
			JOIN (
				SELECT item_key, MAX(seq) AS seq
				FROM fuzz_item_events
				GROUP BY item_key
			) m ON m.item_key = e.item_key AND m.seq = e.seq
		)
		SELECT
			(
				SELECT COUNT(*)
				FROM latest l
				LEFT JOIN fuzz_items i ON i.item_key = l.item_key
				WHERE l.present = 1 AND i.item_key IS NULL
			) AS missing_rows,
			(
				SELECT COUNT(*)
				FROM fuzz_items i
				LEFT JOIN latest l ON l.item_key = i.item_key
				WHERE l.item_key IS NULL OR l.present = 0
			) AS extra_rows,
			(
				SELECT COUNT(*)
				FROM latest l
				JOIN fuzz_items i ON i.item_key = l.item_key
				WHERE l.present = 1
					AND (
						i.value != l.value OR
						i.version != l.version OR
						i.update_count != l.update_count OR
						i.payload_checksum != l.payload_checksum OR
						i.payload_bytes != l.payload_bytes
					)
			) AS mismatched_rows,
			(
				SELECT COUNT(*)
				FROM (
					SELECT item_key
					FROM fuzz_items
					GROUP BY item_key
					HAVING COUNT(*) > 1
				)
			) AS duplicate_keys`,
	);
	const accounts = await queryOne<{
		account_count: number;
		account_balance_sum: number;
		account_balance_mismatch: number;
	}>(
		database,
		`SELECT
			COUNT(*) AS account_count,
			COALESCE(SUM(balance), 0) AS account_balance_sum,
			(
				SELECT COUNT(*)
				FROM fuzz_transfer_events
				WHERE balance_sum_before != ? OR balance_sum_after != ?
			) AS account_balance_mismatch
		FROM fuzz_accounts`,
		ACCOUNT_COUNT * ACCOUNT_INITIAL_BALANCE,
		ACCOUNT_COUNT * ACCOUNT_INITIAL_BALANCE,
	);
	const edge = await queryOne<{
		edge_rows: number;
		edge_expected_rows: number;
		edge_mismatches: number;
	}>(
		database,
		`SELECT
			(SELECT COUNT(*) FROM fuzz_edge_payloads) AS edge_rows,
			(SELECT COUNT(*) FROM fuzz_edge_expectations WHERE present = 1) AS edge_expected_rows,
			(
				SELECT COUNT(*)
				FROM fuzz_edge_expectations e
				LEFT JOIN fuzz_edge_payloads p ON p.id = e.id
				WHERE
					(e.present = 1 AND p.id IS NULL) OR
					(e.present = 0 AND p.id IS NOT NULL) OR
					(e.present = 1 AND (
						p.payload_checksum != e.payload_checksum OR
						p.payload_bytes != e.payload_bytes
					))
			) AS edge_mismatches`,
	);
	const indexProbe = await queryOne<{
		index_rows: number;
		index_mismatches: number;
	}>(
		database,
		`SELECT
			(SELECT COUNT(*) FROM fuzz_indexed) AS index_rows,
			(
				SELECT COUNT(*)
				FROM (
					SELECT id FROM fuzz_indexed
					WHERE tenant = 'tenant-1' AND bucket BETWEEN 2 AND 8 AND score BETWEEN -250 AND 250
					EXCEPT
					SELECT id FROM fuzz_indexed NOT INDEXED
					WHERE tenant = 'tenant-1' AND bucket BETWEEN 2 AND 8 AND score BETWEEN -250 AND 250
				)
			) + (
				SELECT COUNT(*)
				FROM (
					SELECT id FROM fuzz_indexed NOT INDEXED
					WHERE tenant = 'tenant-1' AND bucket BETWEEN 2 AND 8 AND score BETWEEN -250 AND 250
					EXCEPT
					SELECT id FROM fuzz_indexed
					WHERE tenant = 'tenant-1' AND bucket BETWEEN 2 AND 8 AND score BETWEEN -250 AND 250
				)
			) AS index_mismatches`,
	);
	const relational = await queryOne<{
		relational_orders: number;
		relational_mismatches: number;
	}>(
		database,
		`SELECT
			(SELECT COUNT(*) FROM fuzz_orders) AS relational_orders,
			(
				SELECT COUNT(*)
				FROM fuzz_orders o
				LEFT JOIN (
					SELECT order_id, COALESCE(SUM(quantity * price), 0) AS item_total
					FROM fuzz_order_items
					GROUP BY order_id
				) i ON i.order_id = o.id
				WHERE o.total != COALESCE(i.item_total, 0)
			) + (
				SELECT COUNT(*)
				FROM fuzz_orders o
				LEFT JOIN (
					SELECT order_id, COALESCE(SUM(amount), 0) AS payment_total
					FROM fuzz_payments
					WHERE status = 'captured'
					GROUP BY order_id
				) p ON p.order_id = o.id
				WHERE o.status = 'paid' AND o.total != COALESCE(p.payment_total, 0)
			) + (
				SELECT COUNT(*)
				FROM fuzz_inventory
				WHERE initial_qty != sold_qty + stock_qty OR stock_qty < 0
			) AS relational_mismatches`,
	);
	const constraints = await queryOne<{
		constraint_attempts: number;
		constraint_leaks: number;
	}>(
		database,
		`SELECT
			COUNT(*) AS constraint_attempts,
			COALESCE(SUM(
				CASE
					WHEN expected_failed = 1 AND (actually_failed != 1 OR before_count != after_count) THEN 1
					WHEN expected_failed = 0 AND after_count != before_count + 1 THEN 1
					ELSE 0
				END
			), 0) AS constraint_leaks
		FROM fuzz_constraint_attempts`,
	);
	const savepoints = await queryOne<{
		savepoint_rows: number;
		savepoint_mismatches: number;
	}>(
		database,
		`SELECT
			(SELECT COUNT(*) FROM fuzz_savepoints) AS savepoint_rows,
			(
				SELECT COUNT(*)
				FROM fuzz_savepoint_expectations e
				LEFT JOIN fuzz_savepoints s ON s.id = e.id
				WHERE
					(e.present = 1 AND s.id IS NULL) OR
					(e.present = 0 AND s.id IS NOT NULL) OR
					(e.present = 1 AND s.value != e.value)
			) AS savepoint_mismatches`,
	);
	const idempotency = await queryOne<{
		idempotent_ops: number;
		idempotent_mismatches: number;
	}>(
		database,
		`SELECT
			(SELECT COUNT(*) FROM fuzz_idempotent_ops) AS idempotent_ops,
			(
				SELECT COUNT(*)
				FROM fuzz_idempotent_targets t
				LEFT JOIN (
					SELECT target_id, COALESCE(SUM(amount), 0) AS expected
					FROM fuzz_idempotent_ops
					GROUP BY target_id
				) o ON o.target_id = t.id
				WHERE t.value != COALESCE(o.expected, 0)
			) AS idempotent_mismatches`,
	);
	const schema = await queryOne<{
		schema_objects: number;
		schema_missing_objects: number;
	}>(
		database,
		`SELECT
			COUNT(*) AS schema_objects,
			COALESCE(SUM(CASE WHEN m.name IS NULL THEN 1 ELSE 0 END), 0) AS schema_missing_objects
		FROM fuzz_schema_registry r
		LEFT JOIN sqlite_master m ON m.name = r.name AND m.type = r.type`,
	);
	const probes = await queryOne<{
		probe_rows: number;
		probe_mismatches: number;
	}>(
		database,
		`SELECT
			COUNT(*) AS probe_rows,
			COALESCE(SUM(mismatch), 0) AS probe_mismatches
		FROM fuzz_probe_results`,
	);
	const prepared = await queryOne<{
		prepared_rows: number;
		prepared_mismatches: number;
	}>(
		database,
		`SELECT
			(SELECT COUNT(*) FROM fuzz_prepared_churn) AS prepared_rows,
			(
				SELECT COUNT(*)
				FROM fuzz_prepared_expectations e
				LEFT JOIN fuzz_prepared_churn p ON p.id = e.id
				WHERE
					p.id IS NULL OR
					p.value != e.value OR
					p.payload_checksum != e.payload_checksum
			) AS prepared_mismatches`,
	);
	const shadow = await queryOne<{
		shadow_rows: number;
		shadow_mismatches: number;
	}>(
		database,
		`WITH recomputed AS (
			SELECT 'items' AS name,
				COUNT(*) AS row_count,
				COALESCE(SUM(payload_checksum + version + update_count), 0) AS value
			FROM fuzz_items
			UNION ALL
			SELECT 'edge' AS name,
				COUNT(*) AS row_count,
				COALESCE(SUM(payload_checksum + payload_bytes), 0) AS value
			FROM fuzz_edge_payloads
		)
		SELECT
			(SELECT COUNT(*) FROM fuzz_shadow_checksums) AS shadow_rows,
			(
				SELECT COUNT(*)
				FROM fuzz_shadow_checksums s
				JOIN recomputed r ON r.name = s.name
				WHERE s.value != r.value OR s.row_count != r.row_count
			) AS shadow_mismatches`,
	);

	const summary: ValidationSummary = {
		totalEvents: totals?.total_events ?? 0,
		activeRows: totals?.active_rows ?? 0,
		expectedRows: totals?.expected_rows ?? 0,
		missingRows: mismatches?.missing_rows ?? 0,
		extraRows: mismatches?.extra_rows ?? 0,
		mismatchedRows: mismatches?.mismatched_rows ?? 0,
		duplicateKeys: mismatches?.duplicate_keys ?? 0,
		actualVersionSum: totals?.actual_version_sum ?? 0,
		expectedVersionSum: totals?.expected_version_sum ?? 0,
		actualPayloadChecksumSum: totals?.actual_payload_checksum_sum ?? 0,
		expectedPayloadChecksumSum: totals?.expected_payload_checksum_sum ?? 0,
		accountCount: accounts?.account_count ?? 0,
		accountBalanceSum: accounts?.account_balance_sum ?? 0,
		expectedAccountBalanceSum: ACCOUNT_COUNT * ACCOUNT_INITIAL_BALANCE,
		accountBalanceMismatch: accounts?.account_balance_mismatch ?? 0,
		integrityCheck: integrity?.integrity_check ?? "missing",
		quickCheck: quick?.quick_check ?? "missing",
		edgeRows: edge?.edge_rows ?? 0,
		edgeExpectedRows: edge?.edge_expected_rows ?? 0,
		edgeMismatches: edge?.edge_mismatches ?? 0,
		indexRows: indexProbe?.index_rows ?? 0,
		indexMismatches: indexProbe?.index_mismatches ?? 0,
		relationalOrders: relational?.relational_orders ?? 0,
		relationalMismatches: relational?.relational_mismatches ?? 0,
		constraintAttempts: constraints?.constraint_attempts ?? 0,
		constraintLeaks: constraints?.constraint_leaks ?? 0,
		savepointRows: savepoints?.savepoint_rows ?? 0,
		savepointMismatches: savepoints?.savepoint_mismatches ?? 0,
		idempotentOps: idempotency?.idempotent_ops ?? 0,
		idempotentMismatches: idempotency?.idempotent_mismatches ?? 0,
		schemaObjects: schema?.schema_objects ?? 0,
		schemaMissingObjects: schema?.schema_missing_objects ?? 0,
		probeRows: probes?.probe_rows ?? 0,
		probeMismatches: probes?.probe_mismatches ?? 0,
		preparedRows: prepared?.prepared_rows ?? 0,
		preparedMismatches: prepared?.prepared_mismatches ?? 0,
		shadowRows: shadow?.shadow_rows ?? 0,
		shadowMismatches: shadow?.shadow_mismatches ?? 0,
	};

	return summary;
}

async function debugItemMismatches(
	database: { execute: (sql: string, ...args: unknown[]) => Promise<unknown[]> },
	limit = 5,
): Promise<{
	itemMismatches: ItemMismatchDebugRow[];
	recentEventsByKey: Record<string, ItemEventDebugRow[]>;
}> {
	const rows = (await database.execute(
		`WITH latest AS (
			SELECT e.*
			FROM fuzz_item_events e
			JOIN (
				SELECT item_key, MAX(seq) AS seq
				FROM fuzz_item_events
				GROUP BY item_key
			) m ON m.item_key = e.item_key AND m.seq = e.seq
		)
		SELECT
			COALESCE(l.item_key, i.item_key) AS item_key,
			i.value AS actual_value,
			l.value AS expected_value,
			i.version AS actual_version,
			l.version AS expected_version,
			i.update_count AS actual_update_count,
			l.update_count AS expected_update_count,
			i.payload_checksum AS actual_payload_checksum,
			l.payload_checksum AS expected_payload_checksum,
			i.payload_bytes AS actual_payload_bytes,
			l.payload_bytes AS expected_payload_bytes
		FROM latest l
		FULL OUTER JOIN fuzz_items i ON i.item_key = l.item_key
		WHERE
			(l.present = 1 AND i.item_key IS NULL) OR
			((l.item_key IS NULL OR l.present = 0) AND i.item_key IS NOT NULL) OR
			(
				l.present = 1 AND i.item_key IS NOT NULL AND (
					i.value != l.value OR
					i.version != l.version OR
					i.update_count != l.update_count OR
					i.payload_checksum != l.payload_checksum OR
					i.payload_bytes != l.payload_bytes
				)
			)
		ORDER BY COALESCE(l.item_key, i.item_key)
		LIMIT ?`,
		limit,
	)) as Array<{
		item_key: string;
		actual_value: string | null;
		expected_value: string | null;
		actual_version: number | null;
		expected_version: number | null;
		actual_update_count: number | null;
		expected_update_count: number | null;
		actual_payload_checksum: number | null;
		expected_payload_checksum: number | null;
		actual_payload_bytes: number | null;
		expected_payload_bytes: number | null;
	}>;

	const itemMismatches = rows.map((row) => ({
		itemKey: row.item_key,
		actualValue: row.actual_value,
		expectedValue: row.expected_value,
		actualVersion: row.actual_version,
		expectedVersion: row.expected_version,
		actualUpdateCount: row.actual_update_count,
		expectedUpdateCount: row.expected_update_count,
		actualPayloadChecksum: row.actual_payload_checksum,
		expectedPayloadChecksum: row.expected_payload_checksum,
		actualPayloadBytes: row.actual_payload_bytes,
		expectedPayloadBytes: row.expected_payload_bytes,
	}));

	const recentEventsByKey: Record<string, ItemEventDebugRow[]> = {};
	for (const row of itemMismatches) {
		const events = (await database.execute(
			`SELECT
				seq,
				phase,
				local_index,
				kind,
				present,
				value,
				version,
				update_count,
				payload_checksum,
				payload_bytes,
				applied
			FROM fuzz_item_events
			WHERE item_key = ?
			ORDER BY seq DESC
			LIMIT 10`,
			row.itemKey,
		)) as Array<{
			seq: number;
			phase: number;
			local_index: number;
			kind: string;
			present: number;
			value: string | null;
			version: number;
			update_count: number;
			payload_checksum: number;
			payload_bytes: number;
			applied: number;
		}>;
		recentEventsByKey[row.itemKey] = events.map((event) => ({
			seq: event.seq,
			phase: event.phase,
			localIndex: event.local_index,
			kind: event.kind,
			present: event.present,
			value: event.value,
			version: event.version,
			updateCount: event.update_count,
			payloadChecksum: event.payload_checksum,
			payloadBytes: event.payload_bytes,
			applied: event.applied,
		}));
	}

	return { itemMismatches, recentEventsByKey };
}

export const rawSqliteFuzzer = actor({
	options: {
		actionTimeout: 300_000,
	},
	db: db({
		onMigrate: async (database) => {
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_items (
					item_key TEXT PRIMARY KEY,
					value TEXT NOT NULL,
					version INTEGER NOT NULL,
					update_count INTEGER NOT NULL,
					payload TEXT NOT NULL,
					payload_checksum INTEGER NOT NULL,
					payload_bytes INTEGER NOT NULL,
					updated_at INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_item_events (
					seq INTEGER PRIMARY KEY AUTOINCREMENT,
					phase INTEGER NOT NULL,
					local_index INTEGER NOT NULL,
					kind TEXT NOT NULL,
					item_key TEXT NOT NULL,
					present INTEGER NOT NULL,
					value TEXT,
					version INTEGER NOT NULL,
					update_count INTEGER NOT NULL,
					payload_checksum INTEGER NOT NULL,
					payload_bytes INTEGER NOT NULL,
					applied INTEGER NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_fuzz_item_events_key_seq ON fuzz_item_events(item_key, seq)",
			);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_accounts (
					id TEXT PRIMARY KEY,
					balance INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_transfer_events (
					seq INTEGER PRIMARY KEY AUTOINCREMENT,
					phase INTEGER NOT NULL,
					local_index INTEGER NOT NULL,
					from_account TEXT NOT NULL,
					to_account TEXT NOT NULL,
					amount INTEGER NOT NULL,
					balance_sum_before INTEGER NOT NULL,
					balance_sum_after INTEGER NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_edge_payloads (
					id TEXT PRIMARY KEY,
					kind TEXT NOT NULL,
					payload TEXT NOT NULL,
					payload_checksum INTEGER NOT NULL,
					payload_bytes INTEGER NOT NULL,
					updated_at INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_edge_expectations (
					id TEXT PRIMARY KEY,
					present INTEGER NOT NULL,
					payload_checksum INTEGER NOT NULL,
					payload_bytes INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_trigger_audit (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					payload_id TEXT NOT NULL,
					old_checksum INTEGER NOT NULL,
					new_checksum INTEGER NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_fuzz_edge_kind_size ON fuzz_edge_payloads(kind, payload_bytes)",
			);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_indexed (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					tenant TEXT NOT NULL,
					bucket INTEGER NOT NULL,
					score INTEGER NOT NULL,
					label TEXT NOT NULL,
					payload TEXT NOT NULL
				)
			`);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_fuzz_indexed_tenant_bucket_score ON fuzz_indexed(tenant, bucket, score)",
			);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_fuzz_indexed_score_label ON fuzz_indexed(score, label)",
			);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_rel_users (
					id TEXT PRIMARY KEY,
					name TEXT NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_rel_products (
					id TEXT PRIMARY KEY,
					price INTEGER NOT NULL CHECK (price >= 0)
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_orders (
					id TEXT PRIMARY KEY,
					user_id TEXT NOT NULL,
					total INTEGER NOT NULL,
					status TEXT NOT NULL,
					FOREIGN KEY (user_id) REFERENCES fuzz_rel_users(id)
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_order_items (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					order_id TEXT NOT NULL,
					product_id TEXT NOT NULL,
					quantity INTEGER NOT NULL CHECK (quantity > 0),
					price INTEGER NOT NULL CHECK (price >= 0),
					FOREIGN KEY (order_id) REFERENCES fuzz_orders(id),
					FOREIGN KEY (product_id) REFERENCES fuzz_rel_products(id)
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_payments (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					order_id TEXT NOT NULL,
					amount INTEGER NOT NULL,
					status TEXT NOT NULL,
					FOREIGN KEY (order_id) REFERENCES fuzz_orders(id)
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_inventory (
					product_id TEXT PRIMARY KEY,
					initial_qty INTEGER NOT NULL,
					sold_qty INTEGER NOT NULL,
					stock_qty INTEGER NOT NULL,
					FOREIGN KEY (product_id) REFERENCES fuzz_rel_products(id)
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_constraints (
					id TEXT PRIMARY KEY,
					must_not_null TEXT NOT NULL,
					qty INTEGER NOT NULL CHECK (qty >= 0),
					uniq TEXT NOT NULL UNIQUE
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_constraint_attempts (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					name TEXT NOT NULL,
					expected_failed INTEGER NOT NULL,
					actually_failed INTEGER NOT NULL,
					before_count INTEGER NOT NULL,
					after_count INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_fk_parent (
					id TEXT PRIMARY KEY
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_fk_child (
					id TEXT PRIMARY KEY,
					parent_id TEXT NOT NULL,
					FOREIGN KEY (parent_id) REFERENCES fuzz_fk_parent(id) ON DELETE CASCADE
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_savepoints (
					id TEXT PRIMARY KEY,
					value INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_savepoint_expectations (
					id TEXT PRIMARY KEY,
					present INTEGER NOT NULL,
					value INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_idempotent_ops (
					op_id TEXT PRIMARY KEY,
					target_id TEXT NOT NULL,
					amount INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_idempotent_targets (
					id TEXT PRIMARY KEY,
					value INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_schema_registry (
					name TEXT PRIMARY KEY,
					type TEXT NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_probe_results (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					phase INTEGER NOT NULL,
					scenario TEXT NOT NULL,
					name TEXT NOT NULL,
					expected TEXT NOT NULL,
					actual TEXT NOT NULL,
					mismatch INTEGER NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_prepared_churn (
					id TEXT PRIMARY KEY,
					value INTEGER NOT NULL,
					payload TEXT NOT NULL,
					payload_checksum INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_prepared_expectations (
					id TEXT PRIMARY KEY,
					value INTEGER NOT NULL,
					payload_checksum INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_shadow_checksums (
					name TEXT PRIMARY KEY,
					value INTEGER NOT NULL,
					row_count INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_nasty_counter (
					id TEXT PRIMARY KEY,
					value INTEGER NOT NULL
				)
			`);
			await database.execute(`
				CREATE TABLE IF NOT EXISTS fuzz_nasty_rows (
					group_id TEXT NOT NULL,
					n INTEGER NOT NULL,
					payload TEXT NOT NULL,
					PRIMARY KEY (group_id, n)
				)
			`);
		},
	}),
	actions: {
		reset: async (c) => {
			await c.db.execute("DELETE FROM fuzz_nasty_rows");
			await c.db.execute("DELETE FROM fuzz_nasty_counter");
			await c.db.execute("DELETE FROM fuzz_shadow_checksums");
			await c.db.execute("DELETE FROM fuzz_prepared_expectations");
			await c.db.execute("DELETE FROM fuzz_prepared_churn");
			await c.db.execute("DELETE FROM fuzz_probe_results");
			await c.db.execute("DELETE FROM fuzz_schema_registry");
			await c.db.execute("DELETE FROM fuzz_idempotent_targets");
			await c.db.execute("DELETE FROM fuzz_idempotent_ops");
			await c.db.execute("DELETE FROM fuzz_savepoint_expectations");
			await c.db.execute("DELETE FROM fuzz_savepoints");
			await c.db.execute("DELETE FROM fuzz_fk_child");
			await c.db.execute("DELETE FROM fuzz_fk_parent");
			await c.db.execute("DELETE FROM fuzz_constraint_attempts");
			await c.db.execute("DELETE FROM fuzz_constraints");
			await c.db.execute("DELETE FROM fuzz_payments");
			await c.db.execute("DELETE FROM fuzz_order_items");
			await c.db.execute("DELETE FROM fuzz_orders");
			await c.db.execute("DELETE FROM fuzz_inventory");
			await c.db.execute("DELETE FROM fuzz_rel_products");
			await c.db.execute("DELETE FROM fuzz_rel_users");
			await c.db.execute("DELETE FROM fuzz_indexed");
			await c.db.execute("DELETE FROM fuzz_trigger_audit");
			await c.db.execute("DELETE FROM fuzz_edge_expectations");
			await c.db.execute("DELETE FROM fuzz_edge_payloads");
			await c.db.execute("DELETE FROM fuzz_transfer_events");
			await c.db.execute("DELETE FROM fuzz_accounts");
			await c.db.execute("DELETE FROM fuzz_item_events");
			await c.db.execute("DELETE FROM fuzz_items");
			await ensureAccounts(c.db);
			return await validate(c.db);
		},

		runPhase: async (c, input: RunPhaseInput): Promise<PhaseResult> => {
			const mode = input.mode ?? "balanced";
			const iterations = Math.max(1, Math.floor(input.iterations));
			const keySpace = Math.max(1, Math.floor(input.keySpace ?? DEFAULT_KEY_SPACE));
			const maxPayloadBytes = Math.max(
				1,
				Math.floor(input.maxPayloadBytes ?? DEFAULT_MAX_PAYLOAD_BYTES),
			);
			const growthTargetBytes = Math.max(
				1,
				Math.floor(input.growthTargetBytes ?? DEFAULT_GROWTH_TARGET_BYTES),
			);
			const rng = makeRng(`${input.seed}:${input.phase}:${mode}`);
			const ops: Record<string, number> = {};
			let stage = "ensureAccounts";

			try {
				await ensureAccounts(c.db);

				for (let i = 0; i < iterations; i += 1) {
					const kind = chooseKind(mode, rng);
					ops[kind] = (ops[kind] ?? 0) + 1;
					stage = `base:${kind}:iteration:${i}`;

					if (kind === "transfer") {
						const fromIndex = intBetween(rng, 0, ACCOUNT_COUNT - 1);
						let toIndex = intBetween(rng, 0, ACCOUNT_COUNT - 1);
						if (toIndex === fromIndex) toIndex = (toIndex + 1) % ACCOUNT_COUNT;
						const fromAccount = `acct-${fromIndex}`;
						const toAccount = `acct-${toIndex}`;
						try {
							await applyTransfer(c.db, {
								phase: input.phase,
								localIndex: i,
								fromAccount,
								toAccount,
								amount: intBetween(rng, 1, 500),
							});
						} catch (error) {
							throw new Error(
								`base operation transfer failed at iteration ${i} from ${fromAccount} to ${toAccount}`,
								{ cause: error },
							);
						}
					} else if (kind === "hot") {
						const itemKey = `hot-${intBetween(rng, 0, 3)}`;
						const updates = intBetween(rng, 2, mode === "hot" ? 12 : 5);
						try {
							await applyHotUpdates(c.db, {
								seed: input.seed,
								phase: input.phase,
								localIndex: i,
								itemKey,
								updates,
								payloadBytes: intBetween(rng, 1, maxPayloadBytes),
							});
						} catch (error) {
							throw new Error(
								`base operation hot failed at iteration ${i} for ${itemKey} with ${updates} updates`,
								{ cause: error },
							);
						}
					} else {
						const itemKey =
							mode === "hot" && rng() < 0.6
								? `hot-${intBetween(rng, 0, 3)}`
								: `item-${intBetween(rng, 0, keySpace - 1)}`;
						const payloadBytes =
							mode === "payloads"
								? intBetween(rng, Math.min(256, maxPayloadBytes), maxPayloadBytes)
								: intBetween(rng, 1, maxPayloadBytes);
						try {
							await applyItemOperation(c.db, {
								seed: input.seed,
								phase: input.phase,
								localIndex: i,
								kind,
								itemKey,
								payloadBytes,
							});
						} catch (error) {
							throw new Error(
								`base operation ${kind} failed at iteration ${i} for ${JSON.stringify(itemKey)} with payloadBytes ${payloadBytes}`,
								{ cause: error },
							);
						}
					}
				}

				stage = "deep-scenarios";
				await applyDeepScenarios(c.db, {
					seed: input.seed,
					phase: input.phase,
					mode,
					iterations,
					rng,
					maxPayloadBytes,
					growthTargetBytes,
					ops,
				});

				stage = "validate";
				return {
					seed: input.seed,
					phase: input.phase,
					mode,
					iterations,
					ops,
					validation: await validate(c.db),
				};
			} catch (error) {
				const detail =
					error instanceof Error ? error.message : typeof error === "string" ? error : String(error);
				throw new Error(
					`runPhase failed during ${stage} for mode ${mode} phase ${input.phase} seed ${input.seed}: ${detail}`,
					{ cause: error },
				);
			}
		},

		validate: async (c) => {
			await ensureAccounts(c.db);
			return await validate(c.db);
		},

		debugItemMismatches: async (c, limit?: number) => {
			await ensureAccounts(c.db);
			return await debugItemMismatches(c.db, limit ?? 5);
		},

		goToSleep: (c) => {
			c.sleep();
			return { ok: true };
		},
	},
});
