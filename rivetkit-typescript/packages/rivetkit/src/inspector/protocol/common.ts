import z from "zod/v4";
import { ActorKeySchema, MAX_ACTOR_KEY_SIZE } from "@/manager/protocol/query";

export const ActorId = z.string().brand("ActorId");
export type ActorId = z.infer<typeof ActorId>;

export enum ActorFeature {
	Logs = "logs",
	Config = "config",
	Connections = "connections",
	State = "state",
	Console = "console",
	Runtime = "runtime",
	Metrics = "metrics",
	EventsMonitoring = "events-monitoring",
	Database = "database",
}

export const ActorLogEntry = z.object({
	level: z.string(),
	message: z.string(),
	timestamp: z.string(),
	metadata: z.record(z.string(), z.any()).optional(),
});

export const ActorSchema = z.object({
	id: ActorId,
	name: z.string(),
	key: z.array(z.string()),
	tags: z.record(z.string(), z.string()).optional(),
	region: z.string().optional(),
	createdAt: z.string().optional(),
	startedAt: z.string().optional(),
	destroyedAt: z.string().optional(),
	features: z.array(z.enum(ActorFeature)).optional(),
});

export type Actor = z.infer<typeof ActorSchema>;
export type ActorLogEntry = z.infer<typeof ActorLogEntry>;

// MARK: State

export const OperationSchema = z.discriminatedUnion("op", [
	z.object({
		op: z.literal("remove"),
		path: z.string(),
	}),
	z.object({
		op: z.literal("add"),
		path: z.string(),
		value: z.unknown(),
	}),
	z.object({
		op: z.literal("replace"),
		path: z.string(),
		value: z.unknown(),
	}),
	z.object({
		op: z.literal("move"),
		path: z.string(),
		from: z.string(),
	}),
	z.object({
		op: z.literal("copy"),
		path: z.string(),
		from: z.string(),
	}),
	z.object({
		op: z.literal("test"),
		path: z.string(),
		value: z.unknown(),
	}),
]);
export type Operation = z.infer<typeof OperationSchema>;

export const PatchSchema = z.array(OperationSchema);
export type Patch = z.infer<typeof PatchSchema>;

// MARK: Connections

export const ConnectionSchema = z.object({
	params: z.record(z.string(), z.any()).optional(),
	id: z.string(),
	stateEnabled: z.boolean().optional(),
	state: z.any().optional(),
	auth: z.record(z.string(), z.any()).optional(),
});
export type Connection = z.infer<typeof ConnectionSchema>;

// MARK: Realtime Events

export const RealtimeEventSchema = z.discriminatedUnion("type", [
	z.object({
		type: z.literal("action"),
		name: z.string(),
		args: z.array(z.any()),
		connId: z.string(),
	}),
	z.object({
		type: z.literal("broadcast"),
		eventName: z.string(),
		args: z.array(z.any()),
	}),
	z.object({
		type: z.literal("subscribe"),
		eventName: z.string(),
		connId: z.string(),
	}),
	z.object({
		type: z.literal("unsubscribe"),
		eventName: z.string(),
		connId: z.string(),
	}),
	z.object({
		type: z.literal("event"),
		eventName: z.string(),
		args: z.array(z.any()),
		connId: z.string(),
	}),
]);
export type RealtimeEvent = z.infer<typeof RealtimeEventSchema>;
export const RecordedRealtimeEventSchema = RealtimeEventSchema.and(
	z.object({
		id: z.string(),
		timestamp: z.number(),
	}),
);
export type RecordedRealtimeEvent = z.infer<typeof RecordedRealtimeEventSchema>;

// MARK: Database

export const DatabaseQuerySchema = z.object({
	sql: z.string(),
	args: z.array(z.string().or(z.number())),
});
export type DatabaseQuery = z.infer<typeof DatabaseQuerySchema>;

export const TableSchema = z.object({
	schema: z.string(),
	name: z.string(),
	type: z.enum(["table", "view"]),
});
export type Table = z.infer<typeof TableSchema>;

export const TablesSchema = z.array(TableSchema);
export type Tables = z.infer<typeof TablesSchema>;

export const ColumnSchema = z.object({
	cid: z.number(),
	name: z.string(),
	type: z
		.string()
		.toLowerCase()
		.transform((val) => {
			return z
				.enum(["integer", "text", "real", "blob", "numeric", "serial"])
				.parse(val);
		}),
	notnull: z.coerce.boolean(),
	dflt_value: z.string().nullable(),
	pk: z.coerce.boolean().nullable(),
});
export type Column = z.infer<typeof ColumnSchema>;

export const ColumnsSchema = z.array(ColumnSchema);
export type Columns = z.infer<typeof ColumnsSchema>;

export const ForeignKeySchema = z.object({
	id: z.number(),
	table: z.string(),
	from: z.string(),
	to: z.string(),
});
export type ForeignKey = z.infer<typeof ForeignKeySchema>;

export const ForeignKeysSchema = z.array(ForeignKeySchema);
export type ForeignKeys = z.infer<typeof ForeignKeysSchema>;

// MARK: Builds

export const BuildSchema = z.object({
	name: z.string(),
	createdAt: z.string().optional(),
	tags: z.record(z.string(), z.string()).optional(),
});
export type Build = z.infer<typeof BuildSchema>;
export const BuildsSchema = z.array(BuildSchema);
export type Builds = z.infer<typeof BuildsSchema>;

export const CreateActorSchema = z.object({
	name: z.string(),
	// FIXME: Replace with ActorKeySchema when ready
	key: z.array(z.string().max(MAX_ACTOR_KEY_SIZE)),
	input: z.any(),
});
export type CreateActor = z.infer<typeof CreateActorSchema>;
