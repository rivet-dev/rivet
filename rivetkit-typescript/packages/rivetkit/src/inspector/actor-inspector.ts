import { CONN_DRIVER_SYMBOL, CONN_STATE_MANAGER_SYMBOL } from "@/actor/config";
import { RivetError } from "@/actor/errors";
import { Lock } from "@/actor/utils";
import type * as schema from "@/common/bare/generated/inspector/v4";
import { decodeCborCompat, encodeCborCompat } from "@/serde";
import { bufferToArrayBuffer, toUint8Array } from "@/utils";

export interface ActorInspectorWorkflowAdapter {
	getHistory: () => schema.WorkflowHistory | null;
	replayFromStep?: (
		entryId?: string,
	) => Promise<schema.WorkflowHistory | null>;
}

interface InspectorKv {
	get(key: string | Uint8Array): Promise<Uint8Array | null>;
	put(
		key: string | Uint8Array,
		value: string | Uint8Array | ArrayBuffer,
	): Promise<void>;
}

interface InspectorDb {
	execute(sql: string, ...args: Array<unknown>): Promise<unknown>;
}

interface InspectorQueueMessage {
	id: bigint | number;
	name: string;
	createdAt: bigint | number;
}

interface InspectorQueueManager {
	size?: number;
	getMessages(): Promise<Array<InspectorQueueMessage>>;
}

interface InspectorConnStateManager {
	stateEnabled: boolean;
	state?: unknown;
}

type InspectorConnStateSymbolCarrier = {
	[CONN_STATE_MANAGER_SYMBOL]?: InspectorConnStateManager;
};

type InspectorConnDriverSymbolCarrier = {
	[CONN_DRIVER_SYMBOL]?: { type?: string };
};

interface InspectorConnection
	extends InspectorConnStateSymbolCarrier,
		InspectorConnDriverSymbolCarrier {
	params?: unknown;
	subscriptions?: { size: number };
	isHibernatable?: boolean;
	disconnect(reason?: string): Promise<void> | void;
}

interface InspectorConnectionManager {
	connections: Map<string, InspectorConnection>;
	prepareAndConnectConn(
		driver: Record<string, never>,
		param1?: unknown,
		param2?: unknown,
		param3?: unknown,
		param4?: unknown,
	): Promise<InspectorConnection>;
}

interface InspectorStateManager {
	persistRaw: { state: unknown };
	state: unknown;
	saveState(opts: { immediate: boolean }): Promise<void>;
}

interface InspectorConfig {
	options?: {
		maxQueueSize?: number;
	};
}

export interface ActorInspectorActor {
	config: InspectorConfig;
	kv: InspectorKv;
	stateEnabled: boolean;
	stateManager: InspectorStateManager;
	connectionManager: InspectorConnectionManager;
	queueManager: InspectorQueueManager;
	actions: Record<string, unknown>;
	db?: InspectorDb;
	executeAction(
		context: { actor: ActorInspectorActor; conn: InspectorConnection },
		name: string,
		args: unknown[],
	): Promise<unknown>;
}

function createHttpDriver(): Record<string, never> {
	return {};
}

function stateNotEnabledError(): RivetError {
	return new RivetError(
		"actor",
		"state_not_enabled",
		"State not enabled. Must implement `createState` or `state` to use state. (https://www.rivet.dev/docs/actors/state/#initializing-state)",
	);
}

function workflowNotEnabledError(): RivetError {
	return new RivetError(
		"actor",
		"workflow_not_enabled",
		"Workflow not enabled. The run handler must use `workflow(...)` to expose workflow inspector controls.",
	);
}

function databaseNotEnabledError(): RivetError {
	return new RivetError(
		"database",
		"not_enabled",
		"Database not enabled. Must implement `database` to use database.",
	);
}

function encodeCbor(value: unknown): ArrayBuffer {
	return bufferToArrayBuffer(encodeCborCompat(value));
}

function escapeDoubleQuotes(value: string): string {
	return value.replace(/"/g, '""');
}

function toInspectorU64(value: number | bigint): bigint {
	return typeof value === "bigint"
		? value
		: BigInt(Math.max(0, Math.floor(value)));
}

export class ActorInspector {
	#databaseLock = new Lock<void>(undefined);
	#workflow?: ActorInspectorWorkflowAdapter;

	constructor(
		private readonly actor: ActorInspectorActor,
		options?: {
			workflow?: ActorInspectorWorkflowAdapter;
		},
	) {
		this.#workflow = options?.workflow;
	}

	isWorkflowEnabled(): boolean {
		return this.#workflow !== undefined;
	}

	getWorkflowHistory(): schema.WorkflowHistory | null {
		if (!this.#workflow) {
			return null;
		}

		return this.#workflow.getHistory() ?? null;
	}

	async replayWorkflowFromStep(
		entryId?: string,
	): Promise<schema.WorkflowHistory | null> {
		if (!this.#workflow?.replayFromStep) {
			throw workflowNotEnabledError();
		}

		return (await this.#workflow.replayFromStep(entryId)) ?? null;
	}

	isDatabaseEnabled(): boolean {
		return this.actor.db !== undefined;
	}

	isStateEnabled(): boolean {
		return this.actor.stateEnabled;
	}

	getState(): ArrayBuffer {
		if (!this.actor.stateEnabled) {
			throw stateNotEnabledError();
		}

		return encodeCbor(this.actor.stateManager.persistRaw.state);
	}

	getRpcs(): Array<string> {
		return Object.keys(this.actor.actions);
	}

	getConnections(): Array<schema.Connection> {
		return Array.from(
			this.actor.connectionManager.connections.entries(),
		).map(([id, conn]) => {
			const connStateManager = conn[CONN_STATE_MANAGER_SYMBOL];
			return {
				type: conn[CONN_DRIVER_SYMBOL]?.type ?? null,
				id,
				details: encodeCbor({
					type: conn[CONN_DRIVER_SYMBOL]?.type,
					params: conn.params,
					stateEnabled: connStateManager?.stateEnabled ?? false,
					state: connStateManager?.stateEnabled
						? connStateManager.state
						: undefined,
					subscriptions: conn.subscriptions?.size ?? 0,
					isHibernatable: conn.isHibernatable ?? false,
				}),
			};
		});
	}

	async patchState(state: ArrayBuffer | ArrayBufferView): Promise<void> {
		if (!this.actor.stateEnabled) {
			throw stateNotEnabledError();
		}

		this.actor.stateManager.state = decodeCborCompat(toUint8Array(state));
		await this.actor.stateManager.saveState({ immediate: true });
	}

	async executeAction(
		name: string,
		args: ArrayBuffer | ArrayBufferView,
	): Promise<ArrayBuffer> {
		const conn = await this.actor.connectionManager.prepareAndConnectConn(
			createHttpDriver(),
			undefined,
			undefined,
			undefined,
			undefined,
		);

		try {
			const decodedArgs = decodeCborCompat(toUint8Array(args));
			const normalizedArgs = Array.isArray(decodedArgs)
				? decodedArgs
				: [];
			const result = await this.actor.executeAction(
				{ actor: this.actor, conn },
				name,
				normalizedArgs,
			);
			return encodeCbor(result);
		} finally {
			await conn.disconnect();
		}
	}

	async getQueueStatus(limit: number): Promise<schema.QueueStatus> {
		const maxSize = this.actor.config.options?.maxQueueSize ?? 0;
		const safeLimit = Math.max(0, Math.floor(limit));
		const messages = await this.actor.queueManager.getMessages();
		const queueSize = Math.max(
			0,
			Math.floor(this.actor.queueManager.size ?? messages.length),
		);
		const sorted = [...messages].sort((a, b) =>
			Number(toInspectorU64(a.createdAt) - toInspectorU64(b.createdAt)),
		);
		const limited = safeLimit > 0 ? sorted.slice(0, safeLimit) : [];

		return {
			size: BigInt(queueSize),
			maxSize: BigInt(maxSize),
			truncated: sorted.length > limited.length,
			messages: limited.map((message) => ({
				id: toInspectorU64(message.id),
				name: message.name,
				createdAtMs: toInspectorU64(message.createdAt),
			})),
		};
	}

	async getDatabaseSchema(): Promise<ArrayBuffer> {
		return await this.#withDatabase(async (db) => {
			const tables = (await db.execute(
				"SELECT name, type FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '__drizzle_%'",
			)) as Array<{ name: string; type: string }>;

			const tableInfos = [];
			for (const table of tables) {
				const quoted = `"${escapeDoubleQuotes(table.name)}"`;
				const columns = (await db.execute(
					`PRAGMA table_info(${quoted})`,
				)) as Array<{
					cid: number;
					name: string;
					type: string;
					notnull: number;
					dflt_value: string | null;
					pk: number;
				}>;
				const foreignKeys = (await db.execute(
					`PRAGMA foreign_key_list(${quoted})`,
				)) as Array<{
					id: number;
					table: string;
					from: string;
					to: string;
				}>;
				const countResult = (await db.execute(
					`SELECT COUNT(*) as count FROM ${quoted}`,
				)) as Array<{ count: number }>;

				tableInfos.push({
					table: {
						schema: "main",
						name: table.name,
						type: table.type,
					},
					columns: columns.map((column) => ({
						cid: column.cid,
						name: column.name,
						type: column.type,
						notnull: column.notnull,
						dflt_value: column.dflt_value,
						pk: column.pk,
					})),
					foreignKeys: foreignKeys.map((foreignKey) => ({
						id: foreignKey.id,
						table: foreignKey.table,
						from: foreignKey.from,
						to: foreignKey.to,
					})),
					records: countResult.at(0)?.count ?? 0,
				});
			}

			return encodeCbor({ tables: tableInfos });
		});
	}

	async getDatabaseTableRows(
		table: string,
		limit: number,
		offset: number,
	): Promise<ArrayBuffer> {
		return await this.#withDatabase(async (db) => {
			const safeLimit = Math.max(0, Math.min(Math.floor(limit), 500));
			const safeOffset = Math.max(0, Math.floor(offset));
			const quoted = `"${escapeDoubleQuotes(table)}"`;
			const result = await db.execute(
				`SELECT * FROM ${quoted} LIMIT ? OFFSET ?`,
				safeLimit,
				safeOffset,
			);
			return encodeCbor(result);
		});
	}

	async getInit(): Promise<schema.Init> {
		return {
			connections: this.getConnections(),
			state: this.actor.stateEnabled ? this.getState() : null,
			isStateEnabled: this.actor.stateEnabled,
			rpcs: this.getRpcs(),
			isDatabaseEnabled: this.isDatabaseEnabled(),
			queueSize: BigInt(
				Math.max(0, Math.floor(this.actor.queueManager.size ?? 0)),
			),
			workflowHistory: this.getWorkflowHistory(),
			isWorkflowEnabled: this.isWorkflowEnabled(),
		};
	}

	async getStateResponse(rid: bigint): Promise<schema.StateResponse> {
		return {
			rid,
			state: this.actor.stateEnabled ? this.getState() : null,
			isStateEnabled: this.actor.stateEnabled,
		};
	}

	getConnectionsResponse(rid: bigint): schema.ConnectionsResponse {
		return {
			rid,
			connections: this.getConnections(),
		};
	}

	getRpcsListResponse(rid: bigint): schema.RpcsListResponse {
		return {
			rid,
			rpcs: this.getRpcs(),
		};
	}

	async getActionResponse(
		rid: bigint,
		name: string,
		args: ArrayBuffer | ArrayBufferView,
	): Promise<schema.ActionResponse> {
		return {
			rid,
			output: await this.executeAction(name, args),
		};
	}

	async getTraceQueryResponse(
		rid: bigint,
	): Promise<schema.TraceQueryResponse> {
		return {
			rid,
			payload: new ArrayBuffer(0),
		};
	}

	async getQueueResponse(
		rid: bigint,
		limit: number,
	): Promise<schema.QueueResponse> {
		return {
			rid,
			status: await this.getQueueStatus(limit),
		};
	}

	getWorkflowHistoryResponse(rid: bigint): schema.WorkflowHistoryResponse {
		return {
			rid,
			history: this.getWorkflowHistory(),
			isWorkflowEnabled: this.isWorkflowEnabled(),
		};
	}

	async getWorkflowReplayResponse(
		rid: bigint,
		entryId?: string,
	): Promise<schema.WorkflowReplayResponse> {
		return {
			rid,
			history: await this.replayWorkflowFromStep(entryId),
			isWorkflowEnabled: this.isWorkflowEnabled(),
		};
	}

	async getDatabaseSchemaResponse(
		rid: bigint,
	): Promise<schema.DatabaseSchemaResponse> {
		return {
			rid,
			schema: await this.getDatabaseSchema(),
		};
	}

	async getDatabaseTableRowsResponse(
		rid: bigint,
		table: string,
		limit: number,
		offset: number,
	): Promise<schema.DatabaseTableRowsResponse> {
		return {
			rid,
			result: await this.getDatabaseTableRows(table, limit, offset),
		};
	}

	async #withDatabase<T>(fn: (db: InspectorDb) => Promise<T>): Promise<T> {
		if (!this.actor.db) {
			throw databaseNotEnabledError();
		}

		let result: T | undefined;
		await this.#databaseLock.lock(async () => {
			result = await fn(this.actor.db as InspectorDb);
		});
		return result as T;
	}
}
