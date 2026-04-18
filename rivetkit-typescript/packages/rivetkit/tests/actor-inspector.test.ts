import * as cbor from "cbor-x";
import { describe, expect, test } from "vitest";
import { CONN_DRIVER_SYMBOL, CONN_STATE_MANAGER_SYMBOL } from "@/actor/config";
import { KEYS } from "@/actor/keys";
import {
	ActorInspector,
	type ActorInspectorActor,
} from "@/inspector/actor-inspector";
import { bufferToArrayBuffer, toUint8Array } from "@/utils";

function encode(value: unknown): ArrayBuffer {
	return bufferToArrayBuffer(cbor.encode(value));
}

function decode<T>(value: ArrayBuffer | ArrayBufferView): T {
	return cbor.decode(toUint8Array(value)) as T;
}

class MemoryKv {
	store = new Map<string, Uint8Array>();
	lastPutKey?: Uint8Array;

	async get(key: string | Uint8Array): Promise<Uint8Array | null> {
		return this.store.get(this.#key(key)) ?? null;
	}

	async put(
		key: string | Uint8Array,
		value: string | Uint8Array | ArrayBuffer,
	): Promise<void> {
		this.lastPutKey =
			key instanceof Uint8Array ? Uint8Array.from(key) : undefined;
		this.store.set(this.#key(key), this.#value(value));
	}

	#key(key: string | Uint8Array): string {
		if (typeof key === "string") {
			return `str:${key}`;
		}
		return `bytes:${Array.from(key).join(",")}`;
	}

	#value(value: string | Uint8Array | ArrayBuffer): Uint8Array {
		if (typeof value === "string") {
			return new TextEncoder().encode(value);
		}
		if (value instanceof Uint8Array) {
			return Uint8Array.from(value);
		}
		return new Uint8Array(value);
	}
}

function buildActor(): {
	actor: ActorInspectorActor;
	kv: MemoryKv;
	stateManager: {
		persistRaw: { state: unknown };
		state: unknown;
		saveStateCalls: Array<{ immediate: boolean }>;
		saveState(opts: { immediate: boolean }): Promise<void>;
	};
	dbCalls: Array<{ sql: string; args: Array<unknown> }>;
	actionCalls: Array<{ name: string; args: unknown[] }>;
	connectionDisconnects: Array<string>;
} {
	const kv = new MemoryKv();
	const dbCalls: Array<{ sql: string; args: Array<unknown> }> = [];
	const actionCalls: Array<{ name: string; args: unknown[] }> = [];
	const connectionDisconnects: Array<string> = [];
	const stateManager = {
		persistRaw: { state: { count: 2 } },
		state: { count: 2 },
		saveStateCalls: [] as Array<{ immediate: boolean }>,
		async saveState(opts: { immediate: boolean }) {
			this.persistRaw.state = this.state;
			this.saveStateCalls.push(opts);
		},
	};

	const conn = {
		params: { room: "lobby" },
		subscriptions: new Set(["counter.updated", "counter.synced"]),
		isHibernatable: true,
		[CONN_DRIVER_SYMBOL]: { type: "websocket" },
		[CONN_STATE_MANAGER_SYMBOL]: {
			stateEnabled: true,
			state: { connected: true },
		},
		async disconnect(reason?: string) {
			connectionDisconnects.push(reason ?? "");
		},
	};

	const actor: ActorInspectorActor = {
		config: {
			options: {
				maxQueueSize: 1000,
			},
		},
		kv,
		stateEnabled: true,
		stateManager,
		connectionManager: {
			connections: new Map([["conn-1", conn]]),
			async prepareAndConnectConn() {
				return conn;
			},
		},
		queueManager: {
			size: 3,
			async getMessages() {
				return [
					{ id: 2, name: "later", createdAt: 200 },
					{ id: 1, name: "first", createdAt: 100 },
					{ id: 3, name: "last", createdAt: 300 },
				];
			},
		},
		actions: {
			increment: true,
			getCount: true,
		},
		db: {
			async execute(sql: string, ...args: Array<unknown>) {
				dbCalls.push({ sql, args });
				if (sql.includes("sqlite_master")) {
					return [{ name: "widgets", type: "table" }];
				}
				if (sql.startsWith("PRAGMA table_info")) {
					return [
						{
							cid: 0,
							name: "id",
							type: "INTEGER",
							notnull: 1,
							dflt_value: null,
							pk: 1,
						},
					];
				}
				if (sql.startsWith("PRAGMA foreign_key_list")) {
					return [];
				}
				if (sql.startsWith("SELECT COUNT(*)")) {
					return [{ count: 2 }];
				}
				if (sql.startsWith('SELECT * FROM "widgets"')) {
					return [
						{ id: 1, value: "alpha" },
						{ id: 2, value: "beta" },
					];
				}
				throw new Error(`unexpected sql: ${sql}`);
			},
		},
		async executeAction(_context, name, args) {
			actionCalls.push({ name, args });
			return { ok: true, argsLength: args.length };
		},
	};

	return {
		actor,
		kv,
		stateManager,
		dbCalls,
		actionCalls,
		connectionDisconnects,
	};
}

describe("actor inspector", () => {
	test("stores, loads, and verifies inspector tokens at the inspector key", async () => {
		const { actor, kv } = buildActor();
		const inspector = new ActorInspector(actor);

		const token = await inspector.generateToken();

		expect(token.length).toBeGreaterThan(10);
		expect(Array.from(kv.lastPutKey ?? [])).toEqual(
			Array.from(KEYS.INSPECTOR_TOKEN),
		);
		expect(await inspector.loadToken()).toBe(token);
		expect(await inspector.verifyToken(token)).toBe(true);
		expect(await inspector.verifyToken(`${token}-nope`)).toBe(false);
	});

	test("builds init snapshots, queue responses, and workflow responses from actor state", async () => {
		const { actor } = buildActor();
		const history = encode({ steps: ["wake", "run"] });
		const inspector = new ActorInspector(actor, {
			workflow: {
				getHistory: () => history,
				replayFromStep: async (entryId) =>
					encode({ replayedFrom: entryId ?? null }),
			},
		});

		const init = await inspector.getInit();
		const queue = await inspector.getQueueResponse(9n, 2);
		const workflow = inspector.getWorkflowHistoryResponse(10n);
		const replay = await inspector.getWorkflowReplayResponse(
			11n,
			"entry-7",
		);

		expect(init.isStateEnabled).toBe(true);
		expect(init.isDatabaseEnabled).toBe(true);
		expect(init.rpcs).toEqual(["increment", "getCount"]);
		expect(decode(init.state as ArrayBuffer)).toEqual({ count: 2 });
		expect(init.queueSize).toBe(3n);
		expect(init.workflowHistory).toBe(history);
		expect(init.connections).toHaveLength(1);
		expect(decode(init.connections[0].details)).toEqual({
			type: "websocket",
			params: { room: "lobby" },
			stateEnabled: true,
			state: { connected: true },
			subscriptions: 2,
			isHibernatable: true,
		});

		expect(queue).toEqual({
			rid: 9n,
			status: {
				size: 3n,
				maxSize: 1000n,
				truncated: true,
				messages: [
					{ id: 1n, name: "first", createdAtMs: 100n },
					{ id: 2n, name: "later", createdAtMs: 200n },
				],
			},
		});
		expect(workflow).toEqual({
			rid: 10n,
			history,
			isWorkflowEnabled: true,
		});
		expect(decode(replay.history as ArrayBuffer)).toEqual({
			replayedFrom: "entry-7",
		});
	});

	test("patches state immediately, executes actions through a synthetic inspector conn, and serializes database reads", async () => {
		const {
			actor,
			stateManager,
			dbCalls,
			actionCalls,
			connectionDisconnects,
		} = buildActor();
		const inspector = new ActorInspector(actor);

		await inspector.patchState(encode({ count: 9 }));
		const stateResponse = await inspector.getStateResponse(3n);
		const actionResponse = await inspector.getActionResponse(
			4n,
			"increment",
			encode([1, 2, 3]),
		);
		const schemaResponse = await inspector.getDatabaseSchemaResponse(5n);
		const rowsResponse = await inspector.getDatabaseTableRowsResponse(
			6n,
			"widgets",
			10,
			2,
		);
		const traces = await inspector.getTraceQueryResponse(7n);

		expect(stateManager.saveStateCalls).toEqual([{ immediate: true }]);
		expect(stateManager.state).toEqual({ count: 9 });
		expect(decode(stateResponse.state as ArrayBuffer)).toEqual({
			count: 9,
		});

		expect(actionCalls).toEqual([
			{
				name: "increment",
				args: [1, 2, 3],
			},
		]);
		expect(decode(actionResponse.output)).toEqual({
			ok: true,
			argsLength: 3,
		});
		expect(connectionDisconnects).toEqual([""]);

		expect(
			decode<{ tables: Array<{ table: { name: string } }> }>(
				schemaResponse.schema,
			),
		).toEqual({
			tables: [
				{
					table: {
						schema: "main",
						name: "widgets",
						type: "table",
					},
					columns: [
						{
							cid: 0,
							name: "id",
							type: "INTEGER",
							notnull: 1,
							dflt_value: null,
							pk: 1,
						},
					],
					foreignKeys: [],
					records: 2,
				},
			],
		});
		expect(decode(rowsResponse.result)).toEqual([
			{ id: 1, value: "alpha" },
			{ id: 2, value: "beta" },
		]);
		expect(traces).toEqual({ rid: 7n, payload: new ArrayBuffer(0) });
		expect(dbCalls.at(-1)).toEqual({
			sql: 'SELECT * FROM "widgets" LIMIT ? OFFSET ?',
			args: [10, 2],
		});
	});
});
