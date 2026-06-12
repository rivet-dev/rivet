import { actor } from "rivetkit";
import { db } from "rivetkit/db";

const DEFAULT_ROW_BYTES = 2 * 1024;
const ORDER_BATCH_ROWS = 50;
const DOC_BATCH_ROWS = 75;
const LEDGER_BATCH_ROWS = 100;
const POINT_LOOKUP_OPS = 1_000;
const RANGE_CHUNK_ROWS = 512;
const SETUP_TRANSACTION_ROWS = 128;
const FEED_PAGE_ROWS = 100;
const CHAT_LOG_CHUNK_BYTES = 4 * 1024;
const CHAT_LOG_INSERT_BATCH_SIZE = 50;
const CHAT_THREAD_ID = "rw-chat-main";
const SQL_RUSH_MSGS_COUNT = 2_500;
const SQL_RUSH_TOOL_REFS_COUNT = 240;
const SQL_RUSH_EVENTS_COUNT = 700;
const SQL_RUSH_KV_COUNT = 40;
const SQL_RUSH_TOOLS_COUNT = 41;
const SQL_RUSH_META_COUNT = 12;

// Keep this list in sync with the runner's workload catalog. The runner owns
// the per-workload rationale and expected cache/VFS behavior so benchmark
// result artifacts can preserve that intent over time.
const WORKLOADS = [
	"small-rowid-point",
	"small-schema-read",
	"small-range-scan",
	"rowid-range-forward",
	"rowid-range-backward",
	"secondary-index-covering-range",
	"secondary-index-scattered-table",
	"aggregate-status",
	"aggregate-time-bucket",
	"aggregate-tenant-time-range",
	"parallel-read-aggregates",
	"parallel-read-write-transition",
	"feed-order-by-limit",
	"feed-pagination-adjacent",
	"join-order-items",
	"random-point-lookups",
	"hot-index-cold-table",
	"ledger-without-rowid-range",
	"chat-log-select-limit",
	"chat-log-select-indexed",
	"chat-log-count",
	"chat-log-sum",
	"chat-tool-read-fanout",
	"chat-tool-script",
	"write-batch-after-wake",
	"update-hot-partition",
	"delete-churn-range-read",
	"migration-create-indexes-large",
	"migration-create-indexes-skewed-large",
	"migration-table-rebuild-large",
	"migration-add-column-large",
	"migration-ddl-small",
] as const;

type WorkloadName = (typeof WORKLOADS)[number];

interface SetupInput {
	workload: WorkloadName;
	targetBytes?: number;
	rowBytes?: number;
}

interface RunInput {
	workload: WorkloadName;
	targetBytes?: number;
}

interface CountRow {
	rows: number;
}

interface PageCountRow {
	page_count: number;
}

interface CacheSizeRow {
	cache_size: number;
}

interface PageSizeRow {
	page_size: number;
}

interface BytesRow {
	bytes: number;
	rows?: number;
}

interface AggregateRow {
	rows: number;
	total: number;
}

function positiveInteger(
	value: number | undefined,
	fallback: number,
	name: string,
) {
	const resolved = value ?? fallback;
	if (!Number.isInteger(resolved) || resolved < 1) {
		throw new Error(`${name} must be a positive integer`);
	}
	return resolved;
}

function assertWorkload(workload: string): asserts workload is WorkloadName {
	if (!(WORKLOADS as readonly string[]).includes(workload)) {
		throw new Error(`unknown SQLite benchmark workload: ${workload}`);
	}
}

function pseudoRandom(value: number) {
	return Math.imul(value ^ 0x9e3779b9, 0x85ebca6b) >>> 0;
}

function paddedHex(value: number) {
	return pseudoRandom(value).toString(16).padStart(8, "0");
}

function payload(prefix: string, bytes: number) {
	return prefix + "x".repeat(Math.max(0, bytes - prefix.length));
}

function typedRows<T>(rows: unknown[]): T[] {
	return rows as T[];
}

async function queryPageCount(database: {
	execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
}) {
	const [row] = typedRows<PageCountRow>(
		await database.execute("PRAGMA page_count"),
	);
	return row?.page_count ?? 0;
}

async function resetCommerce(database: {
	execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
}) {
	await database.execute("DELETE FROM rw_order_items");
	await database.execute("DELETE FROM rw_orders");
	await database.execute("DELETE FROM rw_customers");
	await database.execute("DELETE FROM rw_events");
}

async function resetDocs(database: {
	execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
}) {
	await database.execute("DELETE FROM rw_docs");
}

async function resetLedger(database: {
	execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
}) {
	await database.execute("DELETE FROM rw_ledger");
}

async function resetChatLog(database: {
	execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
}) {
	await database.execute("DELETE FROM rw_chat_log");
}

async function resetSqlRush(database: {
	execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
}) {
	await database.execute("DELETE FROM tool_refs");
	await database.execute("DELETE FROM msgs");
	await database.execute("DELETE FROM events");
	await database.execute("DELETE FROM kv");
	await database.execute("DELETE FROM tools");
	await database.execute("DELETE FROM meta");
}

async function resetMigration(database: {
	execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
}) {
	await database.execute(
		"DROP INDEX IF EXISTS idx_rw_migration_source_account",
	);
	await database.execute(
		"DROP INDEX IF EXISTS idx_rw_migration_source_created",
	);
	await database.execute(
		"DROP INDEX IF EXISTS idx_rw_migration_source_status_total",
	);
	await database.execute(
		"DROP INDEX IF EXISTS idx_rw_migration_source_skew_account",
	);
	await database.execute(
		"DROP INDEX IF EXISTS idx_rw_migration_source_skew_status",
	);
	await database.execute("DROP TABLE IF EXISTS rw_migration_source_rebuilt");
	await database.execute("DROP TABLE IF EXISTS rw_migration_source");
	await database.execute("DROP TABLE IF EXISTS rw_migration_audit");
	await database.execute("DROP TABLE IF EXISTS rw_migration_empty");
}

async function withTransaction(
	database: {
		execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
	},
	fn: () => Promise<void>,
) {
	let inTransaction = false;
	await database.execute("BEGIN");
	inTransaction = true;
	try {
		await fn();
		await database.execute("COMMIT");
		inTransaction = false;
	} catch (err) {
		if (inTransaction) {
			await database.execute("ROLLBACK").catch(() => undefined);
		}
		throw err;
	}
}

async function seedCommerce(
	database: {
		execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
	},
	targetBytes: number,
	rowBytes: number,
) {
	await resetCommerce(database);
	const rows = Math.max(1, Math.ceil(targetBytes / rowBytes));
	const customerCount = Math.max(32, Math.ceil(rows / 16));
	const startedAt = performance.now();

	await withTransaction(database, async () => {
		for (
			let offset = 0;
			offset < customerCount;
			offset += ORDER_BATCH_ROWS
		) {
			const placeholders: string[] = [];
			const args: unknown[] = [];
			const batchEnd = Math.min(customerCount, offset + ORDER_BATCH_ROWS);
			for (let i = offset; i < batchEnd; i += 1) {
				placeholders.push("(?, ?, ?, ?, ?)");
				args.push(
					i + 1,
					`acct-${i % 64}`,
					`user-${paddedHex(i)}@example.test`,
					["free", "pro", "team", "enterprise"][i % 4],
					["iad", "sfo", "fra", "sin"][i % 4],
				);
			}
			await database.execute(
				`INSERT INTO rw_customers (id, account_id, email, plan, region) VALUES ${placeholders.join(", ")}`,
				...args,
			);
		}
	});

	for (let txStart = 0; txStart < rows; txStart += SETUP_TRANSACTION_ROWS) {
		const txEnd = Math.min(rows, txStart + SETUP_TRANSACTION_ROWS);
		await withTransaction(database, async () => {
			for (
				let offset = txStart;
				offset < txEnd;
				offset += ORDER_BATCH_ROWS
			) {
				const orderPlaceholders: string[] = [];
				const orderArgs: unknown[] = [];
				const itemPlaceholders: string[] = [];
				const itemArgs: unknown[] = [];
				const eventPlaceholders: string[] = [];
				const eventArgs: unknown[] = [];
				const batchEnd = Math.min(txEnd, offset + ORDER_BATCH_ROWS);

				for (let i = offset; i < batchEnd; i += 1) {
					const id = i + 1;
					const customerId = (pseudoRandom(i) % customerCount) + 1;
					const createdAt = 1_700_000_000_000 + i * 1000;
					const status = ["pending", "paid", "shipped", "refunded"][
						i % 4
					];
					const totalCents = 500 + (pseudoRandom(i + 17) % 25_000);
					const note = payload(`order-${id}-${status}:`, rowBytes);

					orderPlaceholders.push("(?, ?, ?, ?, ?, ?, ?)");
					orderArgs.push(
						id,
						customerId,
						createdAt,
						status,
						totalCents,
						i % 128,
						note,
					);

					for (let item = 0; item < 2; item += 1) {
						itemPlaceholders.push("(?, ?, ?, ?, ?)");
						itemArgs.push(
							id,
							`sku-${paddedHex(i + item).slice(0, 6)}`,
							1 + ((i + item) % 5),
							100 + (pseudoRandom(i + item + 31) % 5000),
							item,
						);
					}

					eventPlaceholders.push("(?, ?, ?, ?, ?)");
					eventArgs.push(
						`acct-${customerId % 64}`,
						["click", "purchase", "refund", "shipment"][i % 4],
						createdAt,
						`order:${id}`,
						payload(`event-${id}:`, Math.min(rowBytes, 512)),
					);
				}

				await database.execute(
					`INSERT INTO rw_orders (id, customer_id, created_at, status, total_cents, shard, note) VALUES ${orderPlaceholders.join(", ")}`,
					...orderArgs,
				);
				await database.execute(
					`INSERT INTO rw_order_items (order_id, sku, quantity, price_cents, line_no) VALUES ${itemPlaceholders.join(", ")}`,
					...itemArgs,
				);
				await database.execute(
					`INSERT INTO rw_events (account_id, event_type, created_at, entity_key, properties) VALUES ${eventPlaceholders.join(", ")}`,
					...eventArgs,
				);
			}
		});
	}

	return {
		rows,
		targetBytes,
		rowBytes,
		setupMs: performance.now() - startedAt,
		pageCount: await queryPageCount(database),
	};
}

async function seedDocs(
	database: {
		execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
	},
	targetBytes: number,
	rowBytes: number,
) {
	await resetDocs(database);
	const rows = Math.max(1, Math.ceil(targetBytes / rowBytes));
	const startedAt = performance.now();

	for (let txStart = 0; txStart < rows; txStart += SETUP_TRANSACTION_ROWS) {
		const txEnd = Math.min(rows, txStart + SETUP_TRANSACTION_ROWS);
		await withTransaction(database, async () => {
			for (
				let offset = txStart;
				offset < txEnd;
				offset += DOC_BATCH_ROWS
			) {
				const placeholders: string[] = [];
				const args: unknown[] = [];
				const batchEnd = Math.min(txEnd, offset + DOC_BATCH_ROWS);
				for (let i = offset; i < batchEnd; i += 1) {
					const rank = pseudoRandom(i);
					const body = payload(`doc-${i}-${rank}:`, rowBytes);
					placeholders.push("(?, ?, ?, ?, ?)");
					args.push(
						`doc-${paddedHex(i)}`,
						rank,
						`tenant-${rank % 128}`,
						body,
						rowBytes,
					);
				}
				await database.execute(
					`INSERT INTO rw_docs (external_key, row_rank, tenant_id, body, body_bytes) VALUES ${placeholders.join(", ")}`,
					...args,
				);
			}
		});
	}

	return {
		rows,
		targetBytes,
		rowBytes,
		setupMs: performance.now() - startedAt,
		pageCount: await queryPageCount(database),
	};
}

async function seedLedger(
	database: {
		execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
	},
	targetBytes: number,
	rowBytes: number,
) {
	await resetLedger(database);
	const rows = Math.max(1, Math.ceil(targetBytes / rowBytes));
	const startedAt = performance.now();

	for (let txStart = 0; txStart < rows; txStart += SETUP_TRANSACTION_ROWS) {
		const txEnd = Math.min(rows, txStart + SETUP_TRANSACTION_ROWS);
		await withTransaction(database, async () => {
			for (
				let offset = txStart;
				offset < txEnd;
				offset += LEDGER_BATCH_ROWS
			) {
				const placeholders: string[] = [];
				const args: unknown[] = [];
				const batchEnd = Math.min(txEnd, offset + LEDGER_BATCH_ROWS);
				for (let i = offset; i < batchEnd; i += 1) {
					const accountId = `acct-${String(i % 256).padStart(4, "0")}`;
					const entryId = Math.floor(i / 256) + 1;
					placeholders.push("(?, ?, ?, ?, ?)");
					args.push(
						accountId,
						entryId,
						(i % 2 === 0 ? 1 : -1) * (100 + (i % 10_000)),
						1_700_000_000_000 + i * 1000,
						payload(
							`ledger-${accountId}-${entryId}:`,
							Math.min(rowBytes, 512),
						),
					);
				}
				await database.execute(
					`INSERT INTO rw_ledger (account_id, entry_id, amount_cents, created_at, memo) VALUES ${placeholders.join(", ")}`,
					...args,
				);
			}
		});
	}

	return {
		rows,
		targetBytes,
		rowBytes,
		setupMs: performance.now() - startedAt,
		pageCount: await queryPageCount(database),
	};
}

function buildChatLogMessage(seq: number, targetBytes: number): string {
	const prefix = `message-${seq}: `;
	return prefix + "x".repeat(Math.max(0, targetBytes - prefix.length));
}

async function seedChatLog(
	database: {
		execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
	},
	targetBytes: number,
) {
	await resetChatLog(database);
	const createdAtBase = 1_700_000_000_000;
	let remainingBytes = targetBytes;
	let rows = 0;
	const startedAt = performance.now();

	await withTransaction(database, async () => {
		while (remainingBytes > 0) {
			const placeholders: string[] = [];
			const args: unknown[] = [];

			for (
				let batchIndex = 0;
				batchIndex < CHAT_LOG_INSERT_BATCH_SIZE && remainingBytes > 0;
				batchIndex += 1
			) {
				const contentBytes = Math.min(
					CHAT_LOG_CHUNK_BYTES,
					remainingBytes,
				);
				const seq = rows;
				const role = seq % 2 === 0 ? "user" : "assistant";

				placeholders.push("(?, ?, ?, ?, ?, ?, ?)");
				args.push(
					CHAT_THREAD_ID,
					seq,
					role,
					buildChatLogMessage(seq, contentBytes),
					contentBytes,
					Math.ceil(contentBytes / 4),
					createdAtBase + seq,
				);

				remainingBytes -= contentBytes;
				rows += 1;
			}

			await database.execute(
				`INSERT INTO rw_chat_log (thread_id, seq, role, content, content_bytes, token_estimate, created_at) VALUES ${placeholders.join(", ")}`,
				...args,
			);
		}
	});

	return {
		rows,
		targetBytes,
		rowBytes: CHAT_LOG_CHUNK_BYTES,
		setupMs: performance.now() - startedAt,
		pageCount: await queryPageCount(database),
	};
}

async function batchInsert(
	database: {
		execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
	},
	sql: string,
	rows: unknown[][],
	batchSize: number,
) {
	if (rows.length === 0) return;
	const colsPerRow = rows[0]?.length ?? 0;
	if (colsPerRow === 0) return;
	const placeholder = `(${"?,".repeat(colsPerRow).slice(0, -1)})`;
	for (let i = 0; i < rows.length; i += batchSize) {
		const chunk = rows.slice(i, i + batchSize);
		const values = new Array(chunk.length).fill(placeholder).join(",");
		const args: unknown[] = [];
		for (const row of chunk) args.push(...row);
		await database.execute(`${sql} VALUES ${values}`, ...args);
	}
}

async function seedSqlRush(
	database: {
		execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
	},
	targetBytes: number,
) {
	await resetSqlRush(database);
	const now = 1_700_000_000_000;
	const startedAt = performance.now();

	await withTransaction(database, async () => {
		const msgsRows: unknown[][] = [];
		for (let i = 0; i < SQL_RUSH_MSGS_COUNT; i += 1) {
			msgsRows.push([
				i === 0 ? null : i,
				i % 3 === 0 ? "user" : "assistant",
				payload("msg:", 512),
				0,
				now - (SQL_RUSH_MSGS_COUNT - i) * 1000,
			]);
		}
		await batchInsert(
			database,
			"INSERT INTO msgs (parent, role, content, cancelled, created_at)",
			msgsRows,
			50,
		);

		const toolRefsRows: unknown[][] = [];
		for (let i = 0; i < SQL_RUSH_TOOL_REFS_COUNT; i += 1) {
			toolRefsRows.push([
				i + 1,
				`tool_${i % 20}`,
				`call_${i}`,
				i % 5 === 0 ? "pending" : "done",
			]);
		}
		await batchInsert(
			database,
			"INSERT INTO tool_refs (msg_id, tool_name, tool_call_id, status)",
			toolRefsRows,
			100,
		);

		const eventsRows: unknown[][] = [];
		for (let i = 0; i < SQL_RUSH_EVENTS_COUNT; i += 1) {
			eventsRows.push([
				i + 1,
				`event_${i % 8}`,
				payload("event:", 256),
				now - (SQL_RUSH_EVENTS_COUNT - i) * 100,
			]);
		}
		await batchInsert(
			database,
			"INSERT INTO events (seq, event_type, payload, created_at)",
			eventsRows,
			100,
		);

		const kvRows: unknown[][] = [];
		for (let i = 0; i < SQL_RUSH_KV_COUNT; i += 1) {
			kvRows.push([`kv_${i}`, payload("kv:", 128), now]);
		}
		await batchInsert(
			database,
			"INSERT INTO kv (key, value, updated_at)",
			kvRows,
			40,
		);

		const toolsRows: unknown[][] = [];
		for (let i = 0; i < SQL_RUSH_TOOLS_COUNT; i += 1) {
			toolsRows.push([
				"exec-1",
				`tool_${i}`,
				payload("tool:", 1024),
				now,
			]);
		}
		await batchInsert(
			database,
			"INSERT INTO tools (executor_id, name, spec, updated_at)",
			toolsRows,
			41,
		);

		const metaRows: unknown[][] = [];
		for (let i = 0; i < SQL_RUSH_META_COUNT; i += 1) {
			metaRows.push([`key_${i}`, payload("meta:", 64)]);
		}
		await batchInsert(
			database,
			"INSERT INTO meta (key, value)",
			metaRows,
			12,
		);
	});

	return {
		rows:
			SQL_RUSH_MSGS_COUNT +
			SQL_RUSH_TOOL_REFS_COUNT +
			SQL_RUSH_EVENTS_COUNT +
			SQL_RUSH_KV_COUNT +
			SQL_RUSH_TOOLS_COUNT +
			SQL_RUSH_META_COUNT,
		targetBytes,
		rowBytes: 0,
		setupMs: performance.now() - startedAt,
		pageCount: await queryPageCount(database),
	};
}

async function seedMigrationSource(
	database: {
		execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
	},
	targetBytes: number,
	rowBytes: number,
	skewed = false,
) {
	await resetMigration(database);
	await database.execute(`CREATE TABLE rw_migration_source (
		id INTEGER PRIMARY KEY,
		account_id TEXT NOT NULL,
		status TEXT NOT NULL,
		created_at INTEGER NOT NULL,
		total_cents INTEGER NOT NULL,
		body TEXT NOT NULL
	)`);

	const rows = Math.max(1, Math.ceil(targetBytes / rowBytes));
	const startedAt = performance.now();

	for (let txStart = 0; txStart < rows; txStart += SETUP_TRANSACTION_ROWS) {
		const txEnd = Math.min(rows, txStart + SETUP_TRANSACTION_ROWS);
		await withTransaction(database, async () => {
			for (
				let offset = txStart;
				offset < txEnd;
				offset += ORDER_BATCH_ROWS
			) {
				const placeholders: string[] = [];
				const args: unknown[] = [];
				const batchEnd = Math.min(txEnd, offset + ORDER_BATCH_ROWS);
				for (let i = offset; i < batchEnd; i += 1) {
					const accountId = skewed
						? `acct-${i % 10 === 0 ? i % 512 : i % 8}`
						: `acct-${pseudoRandom(i) % 512}`;
					const status = skewed
						? i % 20 === 0
							? "failed"
							: "open"
						: ["open", "closed", "failed", "pending"][i % 4];
					placeholders.push("(?, ?, ?, ?, ?, ?)");
					args.push(
						i + 1,
						accountId,
						status,
						1_700_000_000_000 + i * 1000,
						100 + (pseudoRandom(i + 41) % 50_000),
						payload(`migration-${i}:`, rowBytes),
					);
				}
				await database.execute(
					`INSERT INTO rw_migration_source (id, account_id, status, created_at, total_cents, body) VALUES ${placeholders.join(", ")}`,
					...args,
				);
			}
		});
	}

	return {
		rows,
		targetBytes,
		rowBytes,
		setupMs: performance.now() - startedAt,
		pageCount: await queryPageCount(database),
	};
}

async function readRowidRange(
	database: {
		execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
	},
	direction: "forward" | "backward",
) {
	const [count] = typedRows<CountRow>(
		await database.execute("SELECT COUNT(*) AS rows FROM rw_orders"),
	);
	const rows = count?.rows ?? 0;
	let bytes = 0;
	let scannedRows = 0;

	if (direction === "backward") {
		for (let upper = rows; upper > 0; upper -= RANGE_CHUNK_ROWS) {
			const lower = Math.max(1, upper - RANGE_CHUNK_ROWS + 1);
			const chunk = typedRows<BytesRow>(
				await database.execute(
					`SELECT length(note) AS bytes FROM rw_orders WHERE id BETWEEN ? AND ? ORDER BY id DESC`,
					lower,
					upper,
				),
			);
			for (const row of chunk) {
				bytes += row.bytes;
				scannedRows += 1;
			}
		}
		return { rows: scannedRows, bytes };
	}

	for (let lower = 1; lower <= rows; lower += RANGE_CHUNK_ROWS) {
		const upper = lower + RANGE_CHUNK_ROWS - 1;
		const [chunk] = typedRows<{ rows: number; bytes: number }>(
			await database.execute(
				`SELECT COUNT(*) AS rows, COALESCE(SUM(length(note)), 0) AS bytes FROM rw_orders WHERE id BETWEEN ? AND ?`,
				lower,
				upper,
			),
		);
		bytes += chunk?.bytes ?? 0;
		scannedRows += chunk?.rows ?? 0;
	}

	return { rows: scannedRows, bytes };
}

export const sqliteRealworldBench = actor({
	options: {
		actionTimeout: 1_200_000,
		sleepGracePeriod: 30_000,
	},
	db: db({
		onMigrate: async (database) => {
			await database.execute(`CREATE TABLE IF NOT EXISTS rw_customers (
				id INTEGER PRIMARY KEY,
				account_id TEXT NOT NULL,
				email TEXT NOT NULL,
				plan TEXT NOT NULL,
				region TEXT NOT NULL
			)`);
			await database.execute(`CREATE TABLE IF NOT EXISTS rw_orders (
				id INTEGER PRIMARY KEY,
				customer_id INTEGER NOT NULL,
				created_at INTEGER NOT NULL,
				status TEXT NOT NULL,
				total_cents INTEGER NOT NULL,
				shard INTEGER NOT NULL,
				note TEXT NOT NULL
			)`);
			await database.execute(`CREATE TABLE IF NOT EXISTS rw_order_items (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				order_id INTEGER NOT NULL,
				sku TEXT NOT NULL,
				quantity INTEGER NOT NULL,
				price_cents INTEGER NOT NULL,
				line_no INTEGER NOT NULL
			)`);
			await database.execute(`CREATE TABLE IF NOT EXISTS rw_events (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				account_id TEXT NOT NULL,
				event_type TEXT NOT NULL,
				created_at INTEGER NOT NULL,
				entity_key TEXT NOT NULL,
				properties TEXT NOT NULL
			)`);
			await database.execute(`CREATE TABLE IF NOT EXISTS rw_docs (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				external_key TEXT NOT NULL UNIQUE,
				row_rank INTEGER NOT NULL,
				tenant_id TEXT NOT NULL,
				body TEXT NOT NULL,
				body_bytes INTEGER NOT NULL
			)`);
			await database.execute(`CREATE TABLE IF NOT EXISTS rw_ledger (
				account_id TEXT NOT NULL,
				entry_id INTEGER NOT NULL,
				amount_cents INTEGER NOT NULL,
				created_at INTEGER NOT NULL,
				memo TEXT NOT NULL,
				PRIMARY KEY (account_id, entry_id)
			) WITHOUT ROWID`);
			await database.execute(`CREATE TABLE IF NOT EXISTS rw_chat_log (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				thread_id TEXT NOT NULL,
				seq INTEGER NOT NULL,
				role TEXT NOT NULL,
				content TEXT NOT NULL,
				content_bytes INTEGER NOT NULL,
				token_estimate INTEGER NOT NULL,
				created_at INTEGER NOT NULL DEFAULT 0
			)`);
			await database.execute(
				"CREATE TABLE IF NOT EXISTS msgs (id INTEGER PRIMARY KEY AUTOINCREMENT, parent INTEGER, role TEXT NOT NULL, content TEXT NOT NULL, cancelled INTEGER NOT NULL DEFAULT 0, created_at INTEGER NOT NULL)",
			);
			await database.execute(
				"CREATE TABLE IF NOT EXISTS tool_refs (id INTEGER PRIMARY KEY AUTOINCREMENT, msg_id INTEGER NOT NULL, tool_name TEXT NOT NULL, tool_call_id TEXT NOT NULL, status TEXT NOT NULL)",
			);
			await database.execute(
				"CREATE TABLE IF NOT EXISTS events (seq INTEGER PRIMARY KEY, event_type TEXT NOT NULL, payload TEXT NOT NULL, created_at INTEGER NOT NULL)",
			);
			await database.execute(
				"CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at INTEGER NOT NULL)",
			);
			await database.execute(
				"CREATE TABLE IF NOT EXISTS tools (id INTEGER PRIMARY KEY AUTOINCREMENT, executor_id TEXT NOT NULL, name TEXT NOT NULL, spec TEXT NOT NULL, updated_at INTEGER NOT NULL)",
			);
			await database.execute(
				"CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
			);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_rw_orders_customer_created ON rw_orders(customer_id, created_at DESC)",
			);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_rw_orders_status_created ON rw_orders(status, created_at)",
			);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_rw_orders_created ON rw_orders(created_at DESC)",
			);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_rw_order_items_order ON rw_order_items(order_id)",
			);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_rw_events_account_created ON rw_events(account_id, created_at)",
			);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_rw_docs_external_rank ON rw_docs(external_key, row_rank)",
			);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_rw_docs_tenant_rank ON rw_docs(tenant_id, row_rank)",
			);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_rw_chat_log_thread_seq ON rw_chat_log(thread_id, seq DESC)",
			);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_msgs_parent_role_cancelled_created_at ON msgs (parent, role, cancelled, created_at)",
			);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_tool_refs_msg_id ON tool_refs (msg_id)",
			);
		},
	}),
	actions: {
		inspectCacheConfig: async (c) => {
			const [cacheSize] = typedRows<CacheSizeRow>(
				await c.db.execute("PRAGMA cache_size"),
			);
			const [pageSize] = typedRows<PageSizeRow>(
				await c.db.execute("PRAGMA page_size"),
			);
			return {
				sqliteCacheSizePragma: cacheSize?.cache_size ?? null,
				sqlitePageSize: pageSize?.page_size ?? null,
				pageCount: await queryPageCount(c.db),
			};
		},

		setupWorkload: async (c, input: SetupInput) => {
			assertWorkload(input.workload);
			const rowBytes = positiveInteger(
				input.rowBytes,
				DEFAULT_ROW_BYTES,
				"rowBytes",
			);
			if (input.workload === "migration-ddl-small") {
				await resetMigration(c.db);
				return {
					rows: 0,
					targetBytes: 0,
					rowBytes,
					setupMs: 0,
					pageCount: await queryPageCount(c.db),
				};
			}
			const targetBytes = positiveInteger(
				input.targetBytes,
				8 * 1024 * 1024,
				"targetBytes",
			);

			switch (input.workload) {
				case "small-rowid-point":
				case "small-schema-read":
				case "small-range-scan":
				case "rowid-range-forward":
				case "rowid-range-backward":
				case "aggregate-status":
				case "aggregate-time-bucket":
				case "aggregate-tenant-time-range":
				case "parallel-read-aggregates":
				case "parallel-read-write-transition":
				case "feed-order-by-limit":
				case "feed-pagination-adjacent":
				case "join-order-items":
				case "random-point-lookups":
				case "write-batch-after-wake":
				case "update-hot-partition":
				case "delete-churn-range-read":
					return seedCommerce(c.db, targetBytes, rowBytes);
				case "secondary-index-covering-range":
				case "secondary-index-scattered-table":
				case "hot-index-cold-table":
					return seedDocs(c.db, targetBytes, rowBytes);
				case "ledger-without-rowid-range":
					return seedLedger(c.db, targetBytes, rowBytes);
				case "chat-log-select-limit":
				case "chat-log-select-indexed":
				case "chat-log-count":
				case "chat-log-sum":
				case "chat-tool-read-fanout":
					return seedChatLog(c.db, targetBytes);
				case "chat-tool-script":
					return seedSqlRush(c.db, targetBytes);
				case "migration-create-indexes-large":
					return seedMigrationSource(c.db, targetBytes, rowBytes);
				case "migration-create-indexes-skewed-large":
					return seedMigrationSource(
						c.db,
						targetBytes,
						rowBytes,
						true,
					);
				case "migration-table-rebuild-large":
				case "migration-add-column-large":
					return seedMigrationSource(c.db, targetBytes, rowBytes);
			}
		},

		runWorkload: async (c, input: RunInput) => {
			assertWorkload(input.workload);
			const t0 = performance.now();
			let details: Record<string, unknown>;

			switch (input.workload) {
				case "small-rowid-point": {
					let bytes = 0;
					for (let i = 0; i < 50; i += 1) {
						const id = (i % 16) + 1;
						const [row] = typedRows<BytesRow>(
							await c.db.execute(
								"SELECT length(note) AS bytes FROM rw_orders WHERE id = ?",
								id,
							),
						);
						bytes += row?.bytes ?? 0;
					}
					details = { ops: 50, bytes };
					break;
				}
				case "small-schema-read": {
					const tables = await c.db.execute(
						"SELECT name, type FROM sqlite_master WHERE type IN ('table', 'index') ORDER BY name",
					);
					const columns = await c.db.execute(
						"PRAGMA table_info(rw_orders)",
					);
					const [count] = typedRows<CountRow>(
						await c.db.execute(
							"SELECT COUNT(*) AS rows FROM rw_orders",
						),
					);
					details = {
						objects: tables.length,
						columns: columns.length,
						rows: count?.rows ?? 0,
					};
					break;
				}
				case "small-range-scan":
				case "rowid-range-forward": {
					details = await readRowidRange(c.db, "forward");
					break;
				}
				case "rowid-range-backward": {
					details = await readRowidRange(c.db, "backward");
					break;
				}
				case "secondary-index-covering-range": {
					const rows = typedRows<{
						external_key: string;
						row_rank: number;
					}>(
						await c.db.execute(
							`SELECT external_key, row_rank FROM rw_docs
						WHERE external_key BETWEEN 'doc-00000000' AND 'doc-ffffffff'
						ORDER BY external_key`,
						),
					);
					let checksum = 0;
					for (const row of rows)
						checksum = (checksum + row.row_rank) >>> 0;
					details = { rows: rows.length, checksum };
					break;
				}
				case "secondary-index-scattered-table": {
					const rows = typedRows<BytesRow>(
						await c.db.execute(
							`SELECT body_bytes AS bytes FROM rw_docs
						WHERE external_key BETWEEN 'doc-00000000' AND 'doc-ffffffff'
						ORDER BY external_key`,
						),
					);
					let bytes = 0;
					for (const row of rows) bytes += row.bytes;
					details = { rows: rows.length, bytes };
					break;
				}
				case "aggregate-status": {
					const rows = typedRows<AggregateRow>(
						await c.db.execute(
							`SELECT status, COUNT(*) AS rows, SUM(total_cents) AS total
						FROM rw_orders
						GROUP BY status
						ORDER BY status`,
						),
					);
					details = {
						groups: rows.length,
						rows: rows.reduce((sum, row) => sum + row.rows, 0),
						total: rows.reduce((sum, row) => sum + row.total, 0),
					};
					break;
				}
				case "aggregate-time-bucket": {
					const rows = typedRows<AggregateRow>(
						await c.db.execute(
							`SELECT (created_at / 300000) AS bucket, COUNT(*) AS rows, SUM(total_cents) AS total
						FROM rw_orders
						GROUP BY bucket
						ORDER BY bucket`,
						),
					);
					details = {
						buckets: rows.length,
						rows: rows.reduce((sum, row) => sum + row.rows, 0),
						total: rows.reduce((sum, row) => sum + row.total, 0),
					};
					break;
				}
				case "aggregate-tenant-time-range": {
					const rows = typedRows<AggregateRow>(
						await c.db.execute(
							`SELECT e.event_type, COUNT(*) AS rows, SUM(o.total_cents) AS total
						FROM rw_events e
						JOIN rw_orders o ON o.id = CAST(substr(e.entity_key, 7) AS INTEGER)
						WHERE e.account_id = ? AND e.created_at BETWEEN ? AND ?
						GROUP BY e.event_type
						ORDER BY e.event_type`,
							"acct-7",
							1_700_000_000_000,
							1_700_000_000_000 + 86_400_000,
						),
					);
					details = {
						groups: rows.length,
						rows: rows.reduce((sum, row) => sum + row.rows, 0),
						total: rows.reduce((sum, row) => sum + row.total, 0),
					};
					break;
				}
				case "parallel-read-aggregates": {
					const [statusRows, bucketRows, tenantRows, joinRows] =
						await Promise.all([
							c.db.execute(
								`SELECT status, COUNT(*) AS rows, SUM(total_cents) AS total
						FROM rw_orders
						GROUP BY status
						ORDER BY status`,
							),
							c.db.execute(
								`SELECT (created_at / 300000) AS bucket, COUNT(*) AS rows, SUM(total_cents) AS total
						FROM rw_orders
						GROUP BY bucket
						ORDER BY bucket`,
							),
							c.db.execute(
								`SELECT e.event_type, COUNT(*) AS rows, SUM(o.total_cents) AS total
						FROM rw_events e
						JOIN rw_orders o ON o.id = CAST(substr(e.entity_key, 7) AS INTEGER)
						WHERE e.account_id = ? AND e.created_at BETWEEN ? AND ?
						GROUP BY e.event_type
						ORDER BY e.event_type`,
								"acct-7",
								1_700_000_000_000,
								1_700_000_000_000 + 86_400_000,
							),
							c.db.execute(
								`SELECT o.status, COUNT(*) AS rows, SUM(oi.quantity * oi.price_cents) AS total
						FROM rw_orders o
						JOIN rw_order_items oi ON oi.order_id = o.id
						GROUP BY o.status
						ORDER BY o.status`,
							),
						]);
					const aggregates = [
						...typedRows<AggregateRow>(statusRows),
						...typedRows<AggregateRow>(bucketRows),
						...typedRows<AggregateRow>(tenantRows),
						...typedRows<AggregateRow>(joinRows),
					];
					details = {
						ops: 4,
						groups: aggregates.length,
						rows: aggregates.reduce(
							(sum, row) => sum + row.rows,
							0,
						),
						total: aggregates.reduce(
							(sum, row) => sum + row.total,
							0,
						),
					};
					break;
				}
				case "parallel-read-write-transition": {
					const readStatus = c.db.execute(
						`SELECT status, COUNT(*) AS rows, SUM(total_cents) AS total
						FROM rw_orders
						GROUP BY status
						ORDER BY status`,
					);
					const readJoin = c.db.execute(
						`SELECT o.status, COUNT(*) AS rows, SUM(oi.quantity * oi.price_cents) AS total
						FROM rw_orders o
						JOIN rw_order_items oi ON oi.order_id = o.id
						GROUP BY o.status
						ORDER BY o.status`,
					);
					const writeHotShard = c.db.execute(
						"UPDATE rw_orders SET total_cents = total_cents + 1 WHERE shard BETWEEN 0 AND 7",
					);
					const readAfterWrite = c.db.execute(
						"SELECT COUNT(*) AS rows FROM rw_orders WHERE shard BETWEEN 0 AND 7",
					);
					const [statusRows, joinRows, , shardRows] =
						await Promise.all([
							readStatus,
							readJoin,
							writeHotShard,
							readAfterWrite,
						]);
					const aggregates = [
						...typedRows<AggregateRow>(statusRows),
						...typedRows<AggregateRow>(joinRows),
					];
					const [shardCount] = typedRows<CountRow>(shardRows);
					details = {
						ops: 4,
						readOps: 3,
						writeOps: 1,
						groups: aggregates.length,
						rows:
							aggregates.reduce((sum, row) => sum + row.rows, 0) +
							(shardCount?.rows ?? 0),
						total: aggregates.reduce(
							(sum, row) => sum + row.total,
							0,
						),
					};
					break;
				}
				case "feed-order-by-limit": {
					const rows = await c.db.execute(
						`SELECT id, customer_id, created_at, status, total_cents
						FROM rw_orders
						WHERE created_at >= ?
						ORDER BY created_at DESC
						LIMIT 1000`,
						1_700_000_000_000,
					);
					details = { rows: rows.length };
					break;
				}
				case "feed-pagination-adjacent": {
					const firstPage = typedRows<{ created_at: number }>(
						await c.db.execute(
							`SELECT created_at
						FROM rw_orders
						WHERE created_at >= ?
						ORDER BY created_at DESC
						LIMIT ?`,
							1_700_000_000_000,
							FEED_PAGE_ROWS,
						),
					);
					const cursor =
						firstPage.at(-1)?.created_at ?? 1_700_000_000_000;
					const secondPage = await c.db.execute(
						`SELECT id, customer_id, created_at, status, total_cents
						FROM rw_orders
						WHERE created_at < ?
						ORDER BY created_at DESC
						LIMIT ?`,
						cursor,
						FEED_PAGE_ROWS,
					);
					details = {
						firstPageRows: firstPage.length,
						rows: secondPage.length,
					};
					break;
				}
				case "join-order-items": {
					const rows = typedRows<AggregateRow>(
						await c.db.execute(
							`SELECT o.status, COUNT(*) AS rows, SUM(oi.quantity * oi.price_cents) AS total
						FROM rw_orders o
						JOIN rw_order_items oi ON oi.order_id = o.id
						GROUP BY o.status
						ORDER BY o.status`,
						),
					);
					details = {
						groups: rows.length,
						rows: rows.reduce((sum, row) => sum + row.rows, 0),
						total: rows.reduce((sum, row) => sum + row.total, 0),
					};
					break;
				}
				case "random-point-lookups": {
					const [count] = typedRows<CountRow>(
						await c.db.execute(
							"SELECT COUNT(*) AS rows FROM rw_orders",
						),
					);
					const rows = Math.max(1, count?.rows ?? 1);
					let bytes = 0;
					for (let i = 0; i < POINT_LOOKUP_OPS; i += 1) {
						const id = (pseudoRandom(i) % rows) + 1;
						const [row] = typedRows<BytesRow>(
							await c.db.execute(
								"SELECT length(note) AS bytes FROM rw_orders WHERE id = ?",
								id,
							),
						);
						bytes += row?.bytes ?? 0;
					}
					details = { ops: POINT_LOOKUP_OPS, bytes };
					break;
				}
				case "hot-index-cold-table": {
					const indexRows = typedRows<{ id: number }>(
						await c.db.execute(
							`SELECT id
						FROM rw_docs
						WHERE tenant_id = ?
						ORDER BY row_rank
						LIMIT 1000`,
							"tenant-7",
						),
					);
					let bytes = 0;
					for (const row of indexRows) {
						const [doc] = typedRows<BytesRow>(
							await c.db.execute(
								"SELECT body_bytes AS bytes FROM rw_docs WHERE id = ?",
								row.id,
							),
						);
						bytes += doc?.bytes ?? 0;
					}
					details = { rows: indexRows.length, bytes };
					break;
				}
				case "ledger-without-rowid-range": {
					const rows = typedRows<BytesRow>(
						await c.db.execute(
							`SELECT account_id, entry_id, amount_cents, length(memo) AS bytes
						FROM rw_ledger
						WHERE account_id BETWEEN 'acct-0040' AND 'acct-0180'
						ORDER BY account_id, entry_id`,
						),
					);
					let bytes = 0;
					for (const row of rows) bytes += row.bytes;
					details = { rows: rows.length, bytes };
					break;
				}
				case "chat-log-select-limit": {
					const rows = await c.db.execute(
						"SELECT seq, role, substr(content, 1, 128) AS preview FROM rw_chat_log ORDER BY created_at DESC LIMIT 100",
					);
					details = { rows: rows.length };
					break;
				}
				case "chat-log-select-indexed": {
					const expectedRows = Math.max(
						1,
						Math.ceil(
							positiveInteger(
								input.targetBytes,
								CHAT_LOG_CHUNK_BYTES,
								"targetBytes",
							) / CHAT_LOG_CHUNK_BYTES,
						),
					);
					const lowerBound = Math.max(0, expectedRows - 100);
					const rows = await c.db.execute(
						"SELECT seq, role, content_bytes FROM rw_chat_log WHERE thread_id = ? AND seq >= ? ORDER BY seq DESC LIMIT 100",
						CHAT_THREAD_ID,
						lowerBound,
					);
					details = { rows: rows.length };
					break;
				}
				case "chat-log-count": {
					const [row] = typedRows<{ count: number }>(
						await c.db.execute(
							"SELECT COUNT(*) AS count FROM rw_chat_log WHERE thread_id = ?",
							CHAT_THREAD_ID,
						),
					);
					details = { ops: 1, rows: row?.count ?? 0 };
					break;
				}
				case "chat-log-sum": {
					const [row] = typedRows<{ total_bytes: number | null }>(
						await c.db.execute(
							"SELECT SUM(content_bytes) AS total_bytes FROM rw_chat_log WHERE thread_id = ?",
							CHAT_THREAD_ID,
						),
					);
					details = { ops: 1, bytes: row?.total_bytes ?? 0 };
					break;
				}
				case "chat-tool-read-fanout": {
					const expectedRows = Math.max(
						1,
						Math.ceil(
							positiveInteger(
								input.targetBytes,
								CHAT_LOG_CHUNK_BYTES,
								"targetBytes",
							) / CHAT_LOG_CHUNK_BYTES,
						),
					);
					const lowerBound = Math.max(0, expectedRows - 100);
					const [limitRows, indexedRows, countRows, sumRows] =
						await Promise.all([
							c.db.execute(
								"SELECT seq, role, substr(content, 1, 128) AS preview FROM rw_chat_log ORDER BY created_at DESC LIMIT 100",
							),
							c.db.execute(
								"SELECT seq, role, content_bytes FROM rw_chat_log WHERE thread_id = ? AND seq >= ? ORDER BY seq DESC LIMIT 100",
								CHAT_THREAD_ID,
								lowerBound,
							),
							c.db.execute(
								"SELECT COUNT(*) AS count FROM rw_chat_log WHERE thread_id = ?",
								CHAT_THREAD_ID,
							),
							c.db.execute(
								"SELECT SUM(content_bytes) AS total_bytes FROM rw_chat_log WHERE thread_id = ?",
								CHAT_THREAD_ID,
							),
						]);
					const [countRow] = typedRows<{ count: number }>(countRows);
					const [sumRow] = typedRows<{ total_bytes: number | null }>(
						sumRows,
					);
					details = {
						ops: 4,
						limitRows: limitRows.length,
						indexedRows: indexedRows.length,
						rows: countRow?.count ?? 0,
						bytes: sumRow?.total_bytes ?? 0,
					};
					break;
				}
				case "chat-tool-script": {
					const [
						msgsRows,
						toolRefsRows,
						eventsRows,
						kvRows,
						toolsRows,
						metaRows,
						unresolvedRows,
					] = await Promise.all([
						c.db.execute(
							"SELECT id, role, length(content) AS bytes FROM msgs WHERE parent IS NOT NULL AND role = ? AND cancelled = 0 ORDER BY created_at DESC LIMIT 50",
							"assistant",
						),
						c.db.execute(
							"SELECT id, tool_name, status FROM tool_refs WHERE status = ? ORDER BY id DESC LIMIT 50",
							"pending",
						),
						c.db.execute(
							"SELECT seq, event_type, length(payload) AS bytes FROM events WHERE seq > ? ORDER BY seq ASC LIMIT 100",
							600,
						),
						c.db.execute(
							"SELECT key, length(value) AS bytes FROM kv ORDER BY updated_at DESC LIMIT 20",
						),
						c.db.execute(
							"SELECT id, name, length(spec) AS bytes FROM tools WHERE executor_id = ? ORDER BY updated_at DESC",
							"exec-1",
						),
						c.db.execute(
							"SELECT key, length(value) AS bytes FROM meta",
						),
						c.db.execute(`SELECT m.id, m.role, count(tr.id) AS pending_refs
							FROM msgs m
							LEFT JOIN tool_refs tr ON tr.msg_id = m.id AND tr.status = 'pending'
							WHERE m.role = 'assistant' AND m.cancelled = 0
							GROUP BY m.id
							ORDER BY m.created_at DESC
							LIMIT 100`),
					]);
					details = {
						ops: 7,
						msgsRows: msgsRows.length,
						toolRefsRows: toolRefsRows.length,
						eventsRows: eventsRows.length,
						kvRows: kvRows.length,
						toolsRows: toolsRows.length,
						metaRows: metaRows.length,
						unresolvedRows: unresolvedRows.length,
					};
					break;
				}
				case "write-batch-after-wake": {
					const [count] = typedRows<CountRow>(
						await c.db.execute(
							"SELECT COUNT(*) AS rows FROM rw_orders",
						),
					);
					const startId = (count?.rows ?? 0) + 1;
					await c.db.execute("BEGIN");
					for (
						let offset = 0;
						offset < 1000;
						offset += ORDER_BATCH_ROWS
					) {
						const placeholders: string[] = [];
						const args: unknown[] = [];
						for (
							let i = offset;
							i < offset + ORDER_BATCH_ROWS;
							i += 1
						) {
							const id = startId + i;
							placeholders.push("(?, ?, ?, ?, ?, ?, ?)");
							args.push(
								id,
								(i % 128) + 1,
								1_800_000_000_000 + i,
								"pending",
								1000 + i,
								i % 128,
								payload(
									`wake-insert-${id}:`,
									DEFAULT_ROW_BYTES,
								),
							);
						}
						await c.db.execute(
							`INSERT INTO rw_orders (id, customer_id, created_at, status, total_cents, shard, note) VALUES ${placeholders.join(", ")}`,
							...args,
						);
					}
					await c.db.execute("COMMIT");
					details = { rows: 1000 };
					break;
				}
				case "update-hot-partition": {
					await c.db.execute(
						"UPDATE rw_orders SET total_cents = total_cents + 1 WHERE shard BETWEEN 0 AND 15",
					);
					const [count] = typedRows<CountRow>(
						await c.db.execute(
							"SELECT COUNT(*) AS rows FROM rw_orders WHERE shard BETWEEN 0 AND 15",
						),
					);
					details = { rows: count?.rows ?? 0 };
					break;
				}
				case "delete-churn-range-read": {
					await c.db.execute(
						"DELETE FROM rw_orders WHERE shard BETWEEN 0 AND 15",
					);
					const result = await readRowidRange(c.db, "forward");
					details = {
						...result,
						deletedShardCount: 16,
					};
					break;
				}
				case "migration-create-indexes-large": {
					await c.db.execute(
						"CREATE INDEX idx_rw_migration_source_account ON rw_migration_source(account_id)",
					);
					await c.db.execute(
						"CREATE INDEX idx_rw_migration_source_created ON rw_migration_source(created_at)",
					);
					await c.db.execute(
						"CREATE INDEX idx_rw_migration_source_status_total ON rw_migration_source(status, total_cents)",
					);
					details = { indexes: 3 };
					break;
				}
				case "migration-create-indexes-skewed-large": {
					await c.db.execute(
						"CREATE INDEX idx_rw_migration_source_skew_account ON rw_migration_source(account_id, created_at)",
					);
					await c.db.execute(
						"CREATE INDEX idx_rw_migration_source_skew_status ON rw_migration_source(status, total_cents)",
					);
					details = { indexes: 2, skewed: true };
					break;
				}
				case "migration-table-rebuild-large": {
					await c.db.execute(`CREATE TABLE rw_migration_source_rebuilt (
						id INTEGER PRIMARY KEY,
						account_id TEXT NOT NULL,
						status TEXT NOT NULL,
						created_at INTEGER NOT NULL,
						total_cents INTEGER NOT NULL,
						body TEXT NOT NULL,
						archived_at INTEGER
					)`);
					await c.db.execute(`INSERT INTO rw_migration_source_rebuilt (
						id, account_id, status, created_at, total_cents, body, archived_at
					)
					SELECT id, account_id, status, created_at, total_cents, body, NULL
					FROM rw_migration_source`);
					await c.db.execute("DROP TABLE rw_migration_source");
					await c.db.execute(
						"ALTER TABLE rw_migration_source_rebuilt RENAME TO rw_migration_source",
					);
					details = { rebuilt: true };
					break;
				}
				case "migration-add-column-large": {
					await c.db.execute(
						"ALTER TABLE rw_migration_source ADD COLUMN archived_at INTEGER",
					);
					details = { alters: 1, rewritesRows: false };
					break;
				}
				case "migration-ddl-small": {
					await c.db.execute(`CREATE TABLE rw_migration_empty (
						id INTEGER PRIMARY KEY,
						tenant_id TEXT NOT NULL,
						created_at INTEGER NOT NULL
					)`);
					await c.db.execute(
						"ALTER TABLE rw_migration_empty ADD COLUMN status TEXT",
					);
					await c.db.execute(
						"CREATE INDEX idx_rw_migration_empty_tenant_created ON rw_migration_empty(tenant_id, created_at)",
					);
					await c.db.execute(`CREATE TABLE rw_migration_audit (
						id INTEGER PRIMARY KEY,
						migration_name TEXT NOT NULL,
						applied_at INTEGER NOT NULL
					)`);
					details = { tables: 2, indexes: 1, alters: 1 };
					break;
				}
			}

			const ms = performance.now() - t0;
			return {
				ms,
				workload: input.workload,
				...details,
				pageCount: await queryPageCount(c.db),
			};
		},

		goToSleep: (c) => {
			c.sleep();
			return { ok: true };
		},
	},
});
