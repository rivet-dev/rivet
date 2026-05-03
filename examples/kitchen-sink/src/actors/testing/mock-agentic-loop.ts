import {
	actor,
	type RivetMessageEvent,
	type UniversalWebSocket,
} from "rivetkit";
import { db } from "rivetkit/db";

const DEFAULT_SLEEP_GRACE_PERIOD_MS = 120_000;
const DEFAULT_ON_SLEEP_DELAY_MS = 15_000;

type EntryRow = {
	request_id: string;
	idx: number;
	created_at: number;
};

type CountRow = {
	count: number;
};

type ExpectedRequest = {
	requestId: string;
	seconds: number;
};

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

function positiveInteger(value: unknown, name: string) {
	if (!Number.isInteger(value) || (value as number) < 1) {
		throw new Error(`${name} must be a positive integer`);
	}

	return value as number;
}

function stringValue(value: unknown, name: string) {
	if (typeof value !== "string" || value.length === 0) {
		throw new Error(`${name} must be a non-empty string`);
	}

	return value;
}

function typedRows<T>(rows: unknown[]): T[] {
	return rows as T[];
}

function numberFromEnv(name: string, fallback: number): number {
	const value = process.env[name];
	if (value === undefined || value === "") return fallback;

	const parsed = Number(value);
	if (!Number.isFinite(parsed) || parsed < 0) {
		throw new Error(`${name} must be a finite non-negative number`);
	}

	return parsed;
}

function send(websocket: UniversalWebSocket, payload: unknown) {
	if (websocket.readyState !== 1) return;
	websocket.send(JSON.stringify(payload));
}

function verifyEntryRows(rows: EntryRow[], expectedSeconds: number) {
	const seen = new Set<number>();
	const indexes = rows.map((row) => row.idx).sort((a, b) => a - b);
	for (const idx of indexes) seen.add(idx);

	const missing: number[] = [];
	for (let idx = 1; idx <= expectedSeconds; idx += 1) {
		if (!seen.has(idx)) missing.push(idx);
	}

	const contiguous =
		rows.length === expectedSeconds &&
		missing.length === 0 &&
		indexes.every((idx, offset) => idx === offset + 1);

	return {
		expectedSeconds,
		count: rows.length,
		contiguous,
		missing,
		indexes,
		ok: contiguous,
	};
}

function verifyAllRows(rows: EntryRow[], expectedRequests: ExpectedRequest[]) {
	const expectedByRequest = new Map(
		expectedRequests.map((request) => [request.requestId, request.seconds]),
	);
	const rowsByRequest = new Map<string, EntryRow[]>();

	for (const row of rows) {
		const requestRows = rowsByRequest.get(row.request_id) ?? [];
		requestRows.push(row);
		rowsByRequest.set(row.request_id, requestRows);
	}

	const requests = expectedRequests.map((request) => {
		const result = verifyEntryRows(
			rowsByRequest.get(request.requestId) ?? [],
			request.seconds,
		);
		return {
			requestId: request.requestId,
			...result,
		};
	});

	const unexpectedRequestIds = [...rowsByRequest.keys()]
		.filter((requestId) => !expectedByRequest.has(requestId))
		.sort();
	const expectedTotalRows = expectedRequests.reduce(
		(total, request) => total + request.seconds,
		0,
	);
	const ok =
		unexpectedRequestIds.length === 0 &&
		rows.length === expectedTotalRows &&
		requests.every((request) => request.ok);

	return {
		type: "verifiedAll",
		expectedRequests: expectedRequests.length,
		expectedTotalRows,
		totalRows: rows.length,
		unexpectedRequestIds,
		requests,
		ok,
	};
}

export const mockAgenticLoop = actor({
	options: {
		canHibernateWebSocket: false,
		sleepGracePeriod: DEFAULT_SLEEP_GRACE_PERIOD_MS,
	},
	db: db({
		onMigrate: async (database) => {
			await database.execute(`
				CREATE TABLE IF NOT EXISTS mock_agentic_entries (
					request_id TEXT NOT NULL,
					idx INTEGER NOT NULL,
					created_at INTEGER NOT NULL,
					PRIMARY KEY (request_id, idx)
				)
			`);
			await database.execute(
				"CREATE INDEX IF NOT EXISTS idx_mock_agentic_entries_created_at ON mock_agentic_entries(created_at)",
			);
		},
	}),
	async onSleep(c) {
		const delayMs = numberFromEnv(
			"MOCK_AGENTIC_ON_SLEEP_DELAY_MS",
			DEFAULT_ON_SLEEP_DELAY_MS,
		);
		c.log.info({
			msg: "mock agentic loop onSleep delay",
			delayMs,
		});
		await sleep(delayMs);
	},
	onRequest(_c, request) {
		const url = new URL(request.url);
		if (url.pathname === "/bypass" || url.pathname === "/request/bypass") {
			return new Response(JSON.stringify({
				type: "bypass",
				transport: "http",
				timestamp: Date.now(),
			}), {
				headers: {
					"content-type": "application/json",
				},
			});
		}

		return new Response("not found", { status: 404 });
	},
	onWebSocket(c, websocket: UniversalWebSocket) {
		const connectionId = crypto.randomUUID();
		let activeInference: Promise<void> | undefined;

		send(websocket, {
			type: "hello",
			connectionId,
			timestamp: Date.now(),
		});

		const verify = async (requestId: string, expectedSeconds: number) => {
			const rows = typedRows<EntryRow>(
				await c.db.execute(
					"SELECT request_id, idx, created_at FROM mock_agentic_entries WHERE request_id = ? ORDER BY idx ASC",
					requestId,
				),
			);
			return {
				type: "verified",
				requestId,
				...verifyEntryRows(rows, expectedSeconds),
			};
		};

		const runInference = async (requestId: string, seconds: number) => {
			send(websocket, {
				type: "started",
				requestId,
				seconds,
				timestamp: Date.now(),
			});

			await c.db.execute(
				"DELETE FROM mock_agentic_entries WHERE request_id = ?",
				requestId,
			);

			for (let idx = 1; idx <= seconds; idx += 1) {
				await sleep(1_000);
				const createdAt = Date.now();
				await c.db.execute(
					"INSERT INTO mock_agentic_entries (request_id, idx, created_at) VALUES (?, ?, ?)",
					requestId,
					idx,
					createdAt,
				);
				send(websocket, {
					type: "progress",
					requestId,
					idx,
					seconds,
					createdAt,
				});
			}

			const verification = await verify(requestId, seconds);
			send(websocket, {
				type: "done",
				requestId,
				seconds,
				timestamp: Date.now(),
				verification,
			});
		};

		websocket.addEventListener("message", (event: RivetMessageEvent) => {
			void (async () => {
				try {
					if (typeof event.data !== "string") {
						throw new Error("message data must be a JSON string");
					}

					const message = JSON.parse(event.data) as Record<
						string,
						unknown
					>;
					const type = stringValue(message.type, "type");

					if (type === "history") {
						const rows = typedRows<EntryRow>(
							await c.db.execute(
								"SELECT request_id, idx, created_at FROM mock_agentic_entries ORDER BY created_at ASC, request_id ASC, idx ASC",
							),
						);
						const [count] = typedRows<CountRow>(
							await c.db.execute(
								"SELECT COUNT(*) AS count FROM mock_agentic_entries",
							),
						);
						send(websocket, {
							type: "history",
							totalRows: count?.count ?? rows.length,
							entries: rows,
							timestamp: Date.now(),
						});
						return;
					}

					if (type === "ping") {
						send(websocket, {
							type: "pong",
							probeId: stringValue(message.probeId, "probeId"),
							timestamp: Date.now(),
						});
						return;
					}

					if (type === "verify") {
						const requestId = stringValue(
							message.requestId,
							"requestId",
						);
						const expectedSeconds = positiveInteger(
							message.expectedSeconds,
							"expectedSeconds",
						);
						send(
							websocket,
							await verify(requestId, expectedSeconds),
						);
						return;
					}

					if (type === "infer") {
						if (activeInference !== undefined) {
							throw new Error("inference already active");
						}

						const requestId = stringValue(
							message.requestId,
							"requestId",
						);
						const seconds = positiveInteger(
							message.seconds,
							"seconds",
						);
						const inference = runInference(
							requestId,
							seconds,
						).finally(() => {
							activeInference = undefined;
						});
						activeInference = inference;
						await c.keepAwake(inference);
						return;
					}

					throw new Error(`unknown message type: ${type}`);
				} catch (error) {
					send(websocket, {
						type: "error",
						message:
							error instanceof Error
								? error.message
								: "unknown websocket error",
						timestamp: Date.now(),
					});
				}
			})();
		});
	},
	actions: {
		verify: async (c, requestId: string, expectedSeconds: number) => {
			const rows = typedRows<EntryRow>(
				await c.db.execute(
					"SELECT request_id, idx, created_at FROM mock_agentic_entries WHERE request_id = ? ORDER BY idx ASC",
					requestId,
				),
			);
			return {
				requestId,
				expectedSeconds,
				count: rows.length,
				indexes: rows.map((row) => row.idx),
			};
		},
		verifyAll: async (c, expectedRequests: ExpectedRequest[]) => {
			if (!Array.isArray(expectedRequests)) {
				throw new Error("expectedRequests must be an array");
			}

			for (const request of expectedRequests) {
				stringValue(request.requestId, "requestId");
				positiveInteger(request.seconds, "seconds");
			}

			const rows = typedRows<EntryRow>(
				await c.db.execute(
					"SELECT request_id, idx, created_at FROM mock_agentic_entries ORDER BY request_id ASC, idx ASC",
				),
			);
			return verifyAllRows(rows, expectedRequests);
		},
	},
});
