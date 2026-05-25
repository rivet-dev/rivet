import { actor, type RivetMessageEvent, type UniversalWebSocket } from "rivetkit";
import { db } from "rivetkit/db";

const DEFAULT_TOKENS_PER_SECOND = 20;
const DEFAULT_DURATION_MS = 5_000;

function send(websocket: UniversalWebSocket, payload: unknown): void {
	if (websocket.readyState !== 1) return;
	websocket.send(JSON.stringify(payload));
}

function parsePositiveNumber(
	value: unknown,
	name: string,
	fallback: number,
): number {
	if (value === undefined || value === null) return fallback;
	const parsed = Number(value);
	if (!Number.isFinite(parsed) || parsed <= 0) {
		throw new Error(`${name} must be a positive number`);
	}
	return parsed;
}

function sleep(ms: number, signal: AbortSignal): Promise<void> {
	if (signal.aborted) return Promise.resolve();
	return new Promise((resolve) => {
		const timeout = setTimeout(resolve, ms);
		signal.addEventListener(
			"abort",
			() => {
				clearTimeout(timeout);
				resolve();
			},
			{ once: true },
		);
	});
}

export const loadTestAgent = actor({
	options: {
		canHibernateWebSocket: false,
		sleepGracePeriod: 5_000,
	},
	db: db({
		onMigrate: async (db) => {
			await db.execute(`
				CREATE TABLE IF NOT EXISTS messages (
					id INTEGER PRIMARY KEY AUTOINCREMENT,
					connection_id TEXT NOT NULL,
					request_id TEXT NOT NULL,
					token_index INTEGER NOT NULL,
					token TEXT NOT NULL,
					created_at INTEGER NOT NULL
				)
			`);
			await db.execute(`
				CREATE INDEX IF NOT EXISTS messages_request_idx
				ON messages (request_id, token_index)
			`);
		},
	}),
	state: {
		connectionCount: 0,
		inferenceCount: 0,
		tokenCount: 0,
	},
	onWebSocket(c, websocket: UniversalWebSocket) {
		c.state.connectionCount += 1;
		const connectionId = crypto.randomUUID();

		send(websocket, {
			type: "connected",
			connectionId,
			connectionCount: c.state.connectionCount,
			timestamp: Date.now(),
		});

		websocket.addEventListener("message", async (event: RivetMessageEvent) => {
			try {
				const message =
					typeof event.data === "string"
						? JSON.parse(event.data)
						: undefined;

				// Fast-path ping: echo back without touching SQLite so the client can measure raw
				// RTT without the per-message storage write. Used by the counter-latency client's
				// first two probes after WS open.
				if (message && message.type === "ping") {
					send(websocket, {
						type: "pong",
						connectionId,
						id: message.id,
						timestamp: Date.now(),
					});
					return;
				}

				if (!message || message.type !== "inference") {
					throw new Error("expected inference message");
				}

				const requestId =
					typeof message.requestId === "string" && message.requestId
						? message.requestId
						: crypto.randomUUID();
				const tokensPerSecond = parsePositiveNumber(
					message.tokensPerSecond,
					"tokensPerSecond",
					DEFAULT_TOKENS_PER_SECOND,
				);
				const durationMs = parsePositiveNumber(
					message.durationMs,
					"durationMs",
					DEFAULT_DURATION_MS,
				);
				const intervalMs = 1_000 / tokensPerSecond;
				const targetTokens = Math.max(
					1,
					Math.floor((durationMs / 1_000) * tokensPerSecond),
				);

				const inference = (async () => {
					c.state.inferenceCount += 1;
					send(websocket, {
						type: "inference-start",
						connectionId,
						requestId,
						tokensPerSecond,
						durationMs,
						targetTokens,
						timestamp: Date.now(),
					});

					const startedAt = performance.now();
					for (let i = 0; i < targetTokens; i++) {
						if (c.abortSignal.aborted || websocket.readyState !== 1) {
							break;
						}

						const tokenIndex = i + 1;
						const token = `token-${tokenIndex}`;
						const createdAt = Date.now();
						await c.db.execute(
							"INSERT INTO messages (connection_id, request_id, token_index, token, created_at) VALUES (?, ?, ?, ?, ?)",
							connectionId,
							requestId,
							tokenIndex,
							token,
							createdAt,
						);
						c.state.tokenCount += 1;

						send(websocket, {
							type: "token",
							connectionId,
							requestId,
							tokenIndex,
							token,
							timestamp: createdAt,
						});

						const nextAt = startedAt + tokenIndex * intervalMs;
						const delayMs = Math.max(0, nextAt - performance.now());
						if (delayMs > 0) {
							await sleep(delayMs, c.abortSignal);
						}
					}

					send(websocket, {
						type: "inference-complete",
						connectionId,
						requestId,
						tokenCount: targetTokens,
						timestamp: Date.now(),
					});
				})();

				await c.keepAwake(inference);
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
		});

		websocket.addEventListener("close", () => {
			c.state.connectionCount -= 1;
		});
	},
	actions: {
		getStats(c) {
			return {
				connectionCount: c.state.connectionCount,
				inferenceCount: c.state.inferenceCount,
				tokenCount: c.state.tokenCount,
			};
		},
	},
});
