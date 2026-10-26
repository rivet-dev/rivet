import {
	actor,
	type RivetMessageEvent,
	type UniversalWebSocket,
} from "rivetkit";
import { db } from "rivetkit/db";

// Low write throughput actor for the v3 load test. A held raw WebSocket keeps
// the actor awake, and each `write` request performs one small KV put or one
// SQLite upsert. Writes rotate over a fixed number of slots so storage stays
// bounded no matter how long the run lasts.
//
// The write counter lives in ephemeral `vars`, so a `kv` write is exactly one KV
// put and a `sqlite` write is exactly one SQLite commit. The `sqlite-state` kind
// also persists `state.writes` after the upsert, which costs a second commit, to
// measure the app pattern of saving actor state after every write.

type WriteKind = "kv" | "sqlite" | "sqlite-state";

type LowWriteRequest =
	| { type: "write"; seq: number; kind: WriteKind; bytes: number }
	| { type: "force_sleep" }
	| { type: "ping"; id?: number };

/** Distinct KV keys or SQLite rows each actor rewrites. */
const WRITE_SLOTS = 16;
/** Upper bound on one write payload. */
const MAX_WRITE_BYTES = 1_048_576;

function send(websocket: UniversalWebSocket, message: object): void {
	if (websocket.readyState === 1) {
		websocket.send(JSON.stringify(message));
	}
}

function jsonResponse(body: object, status = 200): Response {
	return new Response(JSON.stringify(body), {
		status,
		headers: { "Content-Type": "application/json" },
	});
}

function makePayload(bytes: number, seq: number): string {
	const prefix = `${seq}:`;
	if (bytes <= prefix.length) {
		return prefix.slice(0, Math.max(bytes, 0));
	}
	return prefix + "x".repeat(bytes - prefix.length);
}

function clampBytes(bytes: number): number {
	if (!Number.isFinite(bytes) || bytes < 0) {
		throw new Error("bytes must be a non-negative number");
	}
	return Math.min(Math.floor(bytes), MAX_WRITE_BYTES);
}

interface LowWriteContext {
	kv: { put: (key: string, value: string) => Promise<void> };
	db: {
		execute: (sql: string, ...args: unknown[]) => Promise<unknown[]>;
	};
	vars: { writes: number };
	state: { writes: number };
	saveState: (options: { immediate: boolean }) => Promise<void>;
}

async function performWrite(
	c: LowWriteContext,
	kind: WriteKind,
	bytes: number,
	seq: number,
): Promise<void> {
	const payload = makePayload(clampBytes(bytes), seq);
	const slot = seq % WRITE_SLOTS;

	if (kind === "kv") {
		await c.kv.put(`low-write/${slot}`, payload);
	} else {
		// Both SQLite kinds perform the same upsert.
		await c.db.execute(
			`INSERT INTO writes (slot, seq, payload, updated_at) VALUES (?, ?, ?, ?)
			ON CONFLICT(slot) DO UPDATE SET seq = excluded.seq, payload = excluded.payload, updated_at = excluded.updated_at`,
			slot,
			seq,
			payload,
			Date.now(),
		);
	}
	c.vars.writes++;
	if (kind === "sqlite-state") {
		c.state.writes++;
		await c.saveState({ immediate: true });
	}
}

function parseRequest(data: unknown): LowWriteRequest {
	if (typeof data !== "string") {
		throw new Error("low write request must be a string");
	}
	const parsed = JSON.parse(data) as unknown;
	if (!parsed || typeof parsed !== "object") {
		throw new Error("low write request must be an object");
	}
	const request = parsed as Record<string, unknown>;

	if (request.type === "ping") {
		return {
			type: "ping",
			...(typeof request.id === "number" ? { id: request.id } : {}),
		};
	}
	if (request.type === "force_sleep") {
		return { type: "force_sleep" };
	}
	if (request.type === "write") {
		if (
			request.kind !== "kv" &&
			request.kind !== "sqlite" &&
			request.kind !== "sqlite-state"
		) {
			throw new Error("write kind must be kv, sqlite or sqlite-state");
		}
		if (
			typeof request.seq !== "number" ||
			typeof request.bytes !== "number"
		) {
			throw new Error("write requires numeric seq and bytes");
		}
		return {
			type: "write",
			seq: request.seq,
			kind: request.kind,
			bytes: request.bytes,
		};
	}
	throw new Error(`unknown low write request type ${String(request.type)}`);
}

export const lowWriteActor = actor({
	options: {
		canHibernateWebSocket: false,
		sleepGracePeriod: 5_000,
	},
	// Persisted only by the `sqlite-state` kind.
	state: {
		writes: 0,
	},
	createVars: () => ({ writes: 0 }),
	db: db({
		onMigrate: async (database) => {
			await database.execute(`
				CREATE TABLE IF NOT EXISTS writes (
					slot INTEGER PRIMARY KEY,
					seq INTEGER NOT NULL,
					payload TEXT NOT NULL,
					updated_at INTEGER NOT NULL
				)
			`);
		},
	}),
	// First-response probe for the load test. `GET .../probe` runs one SQLite read and
	// returns its timing, so the caller measures create or wake through to an answered
	// request. `GET .../ping` answers without touching storage, so the caller measures
	// the start and request path alone. `GET .../sleep` puts the actor to sleep so the next
	// probe is a wake.
	onRequest: async (c, request) => {
		const receivedAtMs = Date.now();
		const handlerStartedAt = performance.now();
		const pathname = new URL(request.url).pathname;

		if (pathname.endsWith("/soak-write")) {
			const body = (await request.json()) as {
				seq: number;
				kind: WriteKind;
				bytes: number;
			};
			if (
				!Number.isSafeInteger(body.seq) ||
				body.seq < 1 ||
				!["kv", "sqlite"].includes(body.kind)
			) {
				return jsonResponse({ error: "invalid soak write" }, 400);
			}
			await performWrite(c, body.kind, body.bytes, body.seq);
			return jsonResponse({ actorId: c.actorId, seq: body.seq });
		}
		if (pathname.endsWith("/soak-verify")) {
			const body = (await request.json()) as {
				expected: { seq: number; kind: WriteKind; bytes: number }[];
			};
			if (!Array.isArray(body.expected) || body.expected.length > 32) {
				return jsonResponse(
					{ error: "expected at most 32 slot receipts" },
					400,
				);
			}
			const rows = await c.db.execute<{
				slot: number;
				seq: number;
				payload: string;
			}>("SELECT slot, seq, payload FROM writes ORDER BY slot");
			const mismatches: {
				kind: string;
				seq: number;
				actualSeq: number | null;
			}[] = [];
			for (const expected of body.expected) {
				if (
					!Number.isSafeInteger(expected.seq) ||
					expected.seq < 1 ||
					!["kv", "sqlite"].includes(expected.kind)
				) {
					return jsonResponse({ error: "invalid receipt" }, 400);
				}
				const slot = expected.seq % WRITE_SLOTS;
				const row = rows.find((row) => Number(row.slot) === slot);
				const actual =
					expected.kind === "kv"
						? await c.kv.get(`low-write/${slot}`)
						: row?.payload;
				if (
					actual !==
					makePayload(clampBytes(expected.bytes), expected.seq)
				) {
					mismatches.push({
						kind: expected.kind,
						seq: expected.seq,
						actualSeq:
							expected.kind === "kv"
								? null
								: row
									? Number(row.seq)
									: null,
					});
				}
			}
			return jsonResponse({
				actorId: c.actorId,
				verified: body.expected.length,
				mismatches,
			});
		}

		if (pathname.endsWith("/ping")) {
			return jsonResponse({
				type: "ping",
				actorId: c.actorId,
				receivedAtMs,
				handlerMs: performance.now() - handlerStartedAt,
			});
		}
		if (pathname.endsWith("/sleep")) {
			c.sleep();
			return jsonResponse({ type: "sleeping", receivedAtMs });
		}
		if (!pathname.endsWith("/probe")) {
			return jsonResponse({ error: `unknown path ${pathname}` }, 404);
		}

		const readStartedAt = performance.now();
		const rows = await c.db.execute(
			"SELECT slot, seq FROM writes ORDER BY slot LIMIT 4",
		);
		const sqliteReadMs = performance.now() - readStartedAt;
		return jsonResponse({
			type: "probe",
			actorId: c.actorId,
			receivedAtMs,
			rows: rows.length,
			sqliteReadMs,
			handlerMs: performance.now() - handlerStartedAt,
		});
	},
	onWebSocket: (c, websocket: UniversalWebSocket) => {
		send(websocket, {
			type: "connected",
			actorId: c.actorId,
			timestamp: Date.now(),
		});

		websocket.addEventListener("message", (event: RivetMessageEvent) => {
			const handle = async () => {
				let seq: number | undefined;
				try {
					const request = parseRequest(event.data);
					if (request.type === "ping") {
						send(websocket, { type: "pong", id: request.id });
						return;
					}
					if (request.type === "force_sleep") {
						send(websocket, {
							type: "sleeping",
							timestamp: Date.now(),
						});
						c.sleep();
						return;
					}

					seq = request.seq;
					const startedAt = performance.now();
					await performWrite(
						c,
						request.kind,
						request.bytes,
						request.seq,
					);
					send(websocket, {
						type: "write_result",
						seq: request.seq,
						kind: request.kind,
						writeMs: performance.now() - startedAt,
						writes: c.vars.writes,
					});
				} catch (error) {
					send(websocket, {
						type: "write_error",
						...(seq !== undefined ? { seq } : {}),
						error:
							error instanceof Error
								? error.message
								: String(error),
					});
				}
			};
			void c.keepAwake(handle());
		});
	},
	actions: {
		write: async (c, kind: WriteKind = "kv", bytes = 128) => {
			const seq = c.vars.writes;
			await performWrite(c, kind, bytes, seq);
			return { seq, writes: c.vars.writes };
		},
		getWrites: (c) => c.vars.writes,
		sleep: (c) => {
			c.sleep();
			return true;
		},
	},
});
