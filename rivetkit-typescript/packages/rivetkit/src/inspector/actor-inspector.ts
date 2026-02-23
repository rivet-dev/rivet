import * as cbor from "cbor-x";
import { createNanoEvents } from "nanoevents";
import { createHttpDriver } from "@/actor/conn/drivers/http";
import {
	CONN_DRIVER_SYMBOL,
	CONN_STATE_MANAGER_SYMBOL,
} from "@/actor/conn/mod";
import { getRunInspectorConfig } from "@/actor/config";
import { ActionContext } from "@/actor/contexts/action";
import * as actorErrors from "@/actor/errors";
import type { AnyActorInstance } from "@/mod";
import type * as schema from "@/schemas/actor-inspector/mod";
import { bufferToArrayBuffer } from "@/utils";

interface ActorInspectorEmitterEvents {
	stateUpdated: (state: unknown) => void;
	connectionsUpdated: () => void;
	queueUpdated: () => void;
	workflowHistoryUpdated: (history: schema.WorkflowHistory) => void;
}

export type Connection = Omit<schema.Connection, "details"> & {
	details: unknown;
};


/**
 * Provides a unified interface for inspecting actor external and internal state.
 */
export class ActorInspector {
	public readonly emitter = createNanoEvents<ActorInspectorEmitterEvents>();

	#lastQueueSize = 0;
	#workflowInspector?: NonNullable<
		ReturnType<typeof getRunInspectorConfig>
	>["workflow"];

	constructor(private readonly actor: AnyActorInstance) {
		this.#lastQueueSize = actor.queueManager?.size ?? 0;
		const runInspector = getRunInspectorConfig(actor.config.run);
		this.#workflowInspector = runInspector?.workflow;
		if (this.#workflowInspector?.onHistoryUpdated) {
			this.#workflowInspector.onHistoryUpdated((history) => {
				this.emitter.emit(
					"workflowHistoryUpdated",
					history as schema.WorkflowHistory,
				);
			});
		}
	}

	getQueueSize() {
		return this.#lastQueueSize;
	}

	async getQueueStatus(limit: number): Promise<schema.QueueStatus> {
		const maxSize = this.actor.config.options.maxQueueSize;
		const safeLimit = Math.max(0, Math.floor(limit));
		const messages = await this.actor.queueManager.getMessages();
		const sorted = messages.sort(
			(a, b) => a.createdAt - b.createdAt,
		);
		const limited = safeLimit > 0 ? sorted.slice(0, safeLimit) : [];
		return {
			size: BigInt(this.#lastQueueSize),
			maxSize: BigInt(maxSize),
			truncated: sorted.length > limited.length,
			messages: limited.map((message) => ({
				id: message.id,
				name: message.name,
				createdAtMs: BigInt(message.createdAt),
			})),
		};
	}

	updateQueueSize(size: number) {
		if (this.#lastQueueSize === size) {
			return;
		}
		this.#lastQueueSize = size;
		this.emitter.emit("queueUpdated");
	}

	isWorkflowEnabled() {
		return this.#workflowInspector !== undefined;
	}

	getWorkflowHistory(): schema.WorkflowHistory | null {
		if (!this.#workflowInspector) {
			return null;
		}
		const history = this.#workflowInspector.getHistory();
		return (history ?? null) as schema.WorkflowHistory | null;
	}

	// actor accessor methods

	isDatabaseEnabled() {
		try {
			return this.actor.db !== undefined;
		} catch {
			return false;
		}
	}

	async getDatabaseSchema(): Promise<ArrayBuffer> {
		if (!this.isDatabaseEnabled()) {
			throw new actorErrors.DatabaseNotEnabled();
		}

		const db = this.actor.db;

		// Get table list from sqlite_master, excluding internal tables.
		const tables = await db.execute(
			"SELECT name, type FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '__drizzle_%'",
		) as { name: string; type: string }[];

		// Serialize all queries to avoid concurrent @rivetkit/sqlite access
		// which can cause "file is not a database" errors.
		const tableInfos = [];
		for (const table of tables) {
			const quoted = `"${escapeDoubleQuotes(table.name)}"`;
			const sample = await db.execute(
				`SELECT * FROM ${quoted} LIMIT 1`,
			) as Record<string, unknown>[];
			const countResult = await db.execute(
				`SELECT COUNT(*) as count FROM ${quoted}`,
			) as { count: number }[];

			const columnNames = sample?.[0]
				? Object.keys(sample[0])
				: [];

			tableInfos.push({
				table: { schema: "main", name: table.name, type: table.type },
				columns: columnNames.map((name, cid) => ({
					cid,
					name,
					type: "",
					notnull: 0,
					dflt_value: null,
					pk: 0,
				})),
				foreignKeys: [],
				records: countResult?.[0]?.count ?? 0,
			});
		}

		return bufferToArrayBuffer(cbor.encode({ tables: tableInfos }));
	}

	async getDatabaseTableRows(
		table: string,
		limit: number,
		offset: number,
	): Promise<ArrayBuffer> {
		if (!this.isDatabaseEnabled()) {
			throw new actorErrors.DatabaseNotEnabled();
		}

		const db = this.actor.db;
		const safeLimit = Math.max(0, Math.min(Math.floor(limit), 500));
		const safeOffset = Math.max(0, Math.floor(offset));
		const quoted = `"${escapeDoubleQuotes(table)}"`;
		const result = await db.execute(
			`SELECT * FROM ${quoted} LIMIT ? OFFSET ?`,
			safeLimit,
			safeOffset,
		);
		return bufferToArrayBuffer(cbor.encode(result));
	}

	isStateEnabled() {
		return this.actor.stateEnabled;
	}

	getState() {
		if (!this.actor.stateEnabled) {
			throw new actorErrors.StateNotEnabled();
		}
		return bufferToArrayBuffer(
			cbor.encode(this.actor.stateManager.persistRaw.state),
		);
	}

	getRpcs() {
		return this.actor.actions;
	}

	getConnections() {
		return Array.from(
			this.actor.connectionManager.connections.entries(),
		).map(([id, conn]) => {
			const connStateManager = conn[CONN_STATE_MANAGER_SYMBOL];
			return {
				type: conn[CONN_DRIVER_SYMBOL]?.type,
				id,
				details: bufferToArrayBuffer(
					cbor.encode({
						type: conn[CONN_DRIVER_SYMBOL]?.type,
						params: conn.params as any,
						stateEnabled: connStateManager.stateEnabled,
						state: connStateManager.stateEnabled
							? connStateManager.state
							: undefined,
						subscriptions: conn.subscriptions.size,
						isHibernatable: conn.isHibernatable,
						// TODO: Include underlying hibernatable metadata +
						// path + headers
					}),
				),
			};
		});
	}
	async setState(state: ArrayBuffer) {
		if (!this.actor.stateEnabled) {
			throw new actorErrors.StateNotEnabled();
		}
		this.actor.stateManager.state = cbor.decode(Buffer.from(state));
		await this.actor.stateManager.saveState({ immediate: true });
	}

	async executeAction(name: string, params: ArrayBuffer) {
		const conn = await this.actor.connectionManager.prepareAndConnectConn(
			createHttpDriver(),
			// TODO: This may cause issues
			undefined,
			undefined,
			undefined,
			undefined,
		);

		try {
			return bufferToArrayBuffer(
				cbor.encode(
					await this.actor.executeAction(
						new ActionContext(this.actor, conn),
						name,
						cbor.decode(Buffer.from(params)),
					),
				),
			);
		} finally {
			conn.disconnect();
		}
	}

	// JSON-native methods for the HTTP inspector API. These return raw JS
	// objects suitable for JSON serialization instead of CBOR-encoded buffers.

	getStateJson(): unknown {
		if (!this.actor.stateEnabled) {
			throw new actorErrors.StateNotEnabled();
		}
		return this.actor.stateManager.persistRaw.state;
	}

	async setStateJson(state: unknown): Promise<void> {
		if (!this.actor.stateEnabled) {
			throw new actorErrors.StateNotEnabled();
		}
		this.actor.stateManager.state = state;
		await this.actor.stateManager.saveState({ immediate: true });
	}

	getConnectionsJson(): { id: string; details: unknown }[] {
		return Array.from(
			this.actor.connectionManager.connections.entries(),
		).map(([id, conn]) => {
			const connStateManager = conn[CONN_STATE_MANAGER_SYMBOL];
			return {
				type: conn[CONN_DRIVER_SYMBOL]?.type,
				id,
				details: {
					type: conn[CONN_DRIVER_SYMBOL]?.type,
					params: conn.params as any,
					stateEnabled: connStateManager.stateEnabled,
					state: connStateManager.stateEnabled
						? connStateManager.state
						: undefined,
					subscriptions: conn.subscriptions.size,
					isHibernatable: conn.isHibernatable,
				},
			};
		});
	}

	async executeActionJson(name: string, args: unknown[]): Promise<unknown> {
		const conn = await this.actor.connectionManager.prepareAndConnectConn(
			createHttpDriver(),
			undefined,
			undefined,
			undefined,
			undefined,
		);

		try {
			return await this.actor.executeAction(
				new ActionContext(this.actor, conn),
				name,
				args,
			);
		} finally {
			conn.disconnect();
		}
	}

	async getTracesJson(options: {
		startMs: number;
		endMs: number;
		limit: number;
	}): Promise<{ otlp: unknown; clamped: boolean }> {
		const result = await this.actor.traces.readRange(options);
		return result;
	}

	getWorkflowHistoryJson(): { history: unknown | null; isWorkflowEnabled: boolean } {
		const bigIntReplacer = (_key: string, value: unknown) =>
			typeof value === "bigint" ? Number(value) : value;
		const history = this.getWorkflowHistory();
		const safeHistory = history
			? JSON.parse(JSON.stringify(history, bigIntReplacer))
			: null;
		return {
			history: safeHistory,
			isWorkflowEnabled: this.isWorkflowEnabled(),
		};
	}

	getQueueStatusJson(limit: number): Promise<{
		size: number;
		maxSize: number;
		truncated: boolean;
		messages: { id: number; name: string; createdAtMs: number }[];
	}> {
		return this.getQueueStatus(limit).then((status) => ({
			size: Number(status.size),
			maxSize: Number(status.maxSize),
			truncated: status.truncated,
			messages: status.messages.map((m) => ({
				id: Number(m.id),
				name: m.name,
				createdAtMs: Number(m.createdAtMs),
			})),
		}));
	}
}

function escapeDoubleQuotes(value: string): string {
	return value.replace(/"/g, '""');
}

