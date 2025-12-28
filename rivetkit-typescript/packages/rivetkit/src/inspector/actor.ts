import { sValidator } from "@hono/standard-validator";
import jsonPatch from "@rivetkit/fast-json-patch";
import { Hono } from "hono";
import { streamSSE } from "hono/streaming";
import { createNanoEvents, type Unsubscribe } from "nanoevents";
import z from "zod/v4";
import type {
	AnyDatabaseProvider,
	InferDatabaseClient,
} from "@/actor/database";
import { promiseWithResolvers } from "@/utils";
import {
	ColumnsSchema,
	type Connection,
	ForeignKeysSchema,
	PatchSchema,
	type RealtimeEvent,
	type RecordedRealtimeEvent,
	TablesSchema,
} from "./protocol/common";

export type ActorInspectorRouterEnv = {
	Variables: {
		inspector: ActorInspector;
	};
};

/**
 * Create a router for the Actor Inspector.
 * @internal
 */
export function createActorInspectorRouter() {
	return new Hono<ActorInspectorRouterEnv>()
		.get("/ping", (c) => {
			return c.json({ message: "pong" }, 200);
		})
		.get("/state", async (c) => {
			if (await c.var.inspector.accessors.isStateEnabled()) {
				return c.json(
					{
						enabled: true,
						state: await c.var.inspector.accessors.getState(),
					},
					200,
				);
			}
			return c.json({ enabled: false, state: null }, 200);
		})
		.patch(
			"/state",
			sValidator(
				"json",
				z
					.object({ patch: PatchSchema })
					.or(z.object({ replace: z.any() })),
			),
			async (c) => {
				if (!(await c.var.inspector.accessors.isStateEnabled())) {
					return c.json({ enabled: false }, 200);
				}

				const body = c.req.valid("json");
				if ("replace" in body) {
					await c.var.inspector.accessors.setState(body.replace);
					return c.json(
						{
							enabled: true,
							state: await c.var.inspector.accessors.getState(),
						},
						200,
					);
				}
				const state = await c.var.inspector.accessors.getState();

				const { newDocument: newState } = jsonPatch.applyPatch(
					state,
					body.patch,
				);
				await c.var.inspector.accessors.setState(newState);

				return c.json(
					{
						enabled: true,
						state: await c.var.inspector.accessors.getState(),
					},
					200,
				);
			},
		)
		.get("/state/stream", async (c) => {
			if (!(await c.var.inspector.accessors.isStateEnabled())) {
				return c.json({ enabled: false }, 200);
			}

			let id = 0;
			let unsub: Unsubscribe;
			return streamSSE(
				c,
				async (stream) => {
					unsub = c.var.inspector.emitter.on(
						"stateUpdated",
						async (state) => {
							stream.writeSSE({
								data: JSON.stringify(state) || "",
								event: "state-update",
								id: String(id++),
							});
						},
					);

					const { promise } = promiseWithResolvers<void>();

					return promise;
				},
				async () => {
					unsub?.();
				},
			);
		})
		.get("/connections", async (c) => {
			const connections =
				await c.var.inspector.accessors.getConnections();
			return c.json({ connections }, 200);
		})
		.get("/connections/stream", async (c) => {
			let id = 0;
			let unsub: Unsubscribe;
			return streamSSE(
				c,
				async (stream) => {
					unsub = c.var.inspector.emitter.on(
						"connectionUpdated",
						async () => {
							stream.writeSSE({
								data: JSON.stringify(
									await c.var.inspector.accessors.getConnections(),
								),
								event: "connection-update",
								id: String(id++),
							});
						},
					);

					const { promise } = promiseWithResolvers<void>();

					return promise;
				},
				async () => {
					unsub?.();
				},
			);
		})
		.get("/events", async (c) => {
			const events = c.var.inspector.lastRealtimeEvents;
			return c.json({ events }, 200);
		})
		.post("/events/clear", async (c) => {
			c.var.inspector.lastRealtimeEvents.length = 0; // Clear the events
			return c.json({ message: "Events cleared" }, 200);
		})
		.get("/events/stream", async (c) => {
			let id = 0;
			let unsub: Unsubscribe;
			return streamSSE(
				c,
				async (stream) => {
					unsub = c.var.inspector.emitter.on("eventFired", () => {
						stream.writeSSE({
							data: JSON.stringify(
								c.var.inspector.lastRealtimeEvents,
							),
							event: "realtime-event",
							id: String(id++),
						});
					});

					const { promise } = promiseWithResolvers<void>();

					return promise;
				},
				async () => {
					unsub?.();
				},
			);
		})
		.get("/rpcs", async (c) => {
			const rpcs = await c.var.inspector.accessors.getRpcs();
			return c.json({ rpcs }, 200);
		})
		.get("/db", async (c) => {
			if (!(await c.var.inspector.accessors.isDbEnabled())) {
				return c.json({ enabled: false, db: null }, 200);
			}

			// Access the SQLite database
			const db = await c.var.inspector.accessors.getDb();

			// Get list of tables
			const rows = await db.execute(`PRAGMA table_list`);
			const tables = TablesSchema.parse(rows).filter(
				(table) =>
					table.schema !== "temp" &&
					!table.name.startsWith("sqlite_"),
			);
			// Get columns for each table
			const tablesInfo = await Promise.all(
				tables.map((table) =>
					db.execute(`PRAGMA table_info(${table.name})`),
				),
			);
			const columns = tablesInfo.map((def) => ColumnsSchema.parse(def));

			// Get foreign keys for each table
			const foreignKeysList = await Promise.all(
				tables.map((table) =>
					db.execute(`PRAGMA foreign_key_list(${table.name})`),
				),
			);
			const foreignKeys = foreignKeysList.map((def) =>
				ForeignKeysSchema.parse(def),
			);

			// Get record counts for each table
			const countInfo = await Promise.all(
				tables.map((table) =>
					db.execute(`SELECT COUNT(*) as count FROM ${table.name}`),
				),
			);
			const counts = countInfo.map((def) => {
				return def[0].count || 0;
			});

			return c.json(
				{
					enabled: true,
					db: tablesInfo.map((_, index) => {
						return {
							table: tables[index],
							columns: columns[index],
							foreignKeys: foreignKeys[index],
							records: counts[index],
						};
					}),
				},
				200,
			);
		})
		.post(
			"/db",
			sValidator(
				"json",
				z.object({
					query: z.string(),
					params: z.array(z.any()).optional(),
				}),
			),
			async (c) => {
				if (!(await c.var.inspector.accessors.isDbEnabled())) {
					return c.json({ enabled: false }, 200);
				}
				const db = await c.var.inspector.accessors.getDb();

				try {
					const result = (await db.execute(
						c.req.valid("json").query,
						...(c.req.valid("json").params || []),
					)) as unknown;
					return c.json({ result }, 200);
				} catch (error) {
					c;
					return c.json({ error: (error as Error).message }, 500);
				}
			},
		)
		.post(
			"/action",
			sValidator(
				"json",
				z.object({
					name: z.string(),
					params: z.array(z.any()).optional(),
				}),
			),
			async (c) => {
				const { name, params } = c.req.valid("json");
				const result = await c.var.inspector.accessors.executeAction(
					name,
					params,
				);
				return c.json({ result }, 200);
			},
		);
}

interface ActorInspectorAccessors {
	isStateEnabled: () => Promise<boolean>;
	getState: () => Promise<unknown>;
	setState: (state: unknown) => Promise<void>;
	isDbEnabled: () => Promise<boolean>;
	getDb: () => Promise<InferDatabaseClient<AnyDatabaseProvider>>;
	getRpcs: () => Promise<string[]>;
	getConnections: () => Promise<Connection[]>;
	executeAction: (name: string, params?: unknown[]) => Promise<unknown>;
	getKvEntries: (options: {
		prefix?: string;
		limit?: number;
		reverse?: boolean;
	}) => Promise<
		Array<{
			key: string;
			value: string;
			updateTs: number;
		}>
	>;
}

interface ActorInspectorEmitterEvents {
	stateUpdated: (state: unknown) => void;
	connectionUpdated: () => void;
	eventFired: (event: RealtimeEvent) => void;
}

/**
 * Provides a unified interface for inspecting actor external and internal state.
 */
export class ActorInspector {
	public readonly accessors: ActorInspectorAccessors;
	public readonly emitter = createNanoEvents<ActorInspectorEmitterEvents>();

	#lastRealtimeEvents: RecordedRealtimeEvent[] = [];

	get lastRealtimeEvents() {
		return this.#lastRealtimeEvents;
	}

	constructor(accessors: () => ActorInspectorAccessors) {
		this.accessors = accessors();
		this.emitter.on("eventFired", (event) => {
			this.#lastRealtimeEvents.push({
				id: crypto.randomUUID(),
				timestamp: Date.now(),
				...event,
			});
			// keep the last 100 events
			if (this.#lastRealtimeEvents.length > 100) {
				this.#lastRealtimeEvents = this.#lastRealtimeEvents.slice(-100);
			}
		});
	}
}
