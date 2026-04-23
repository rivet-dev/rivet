import type {
	ActionContext,
	BeforeActionResponseContext,
	BeforeConnectContext,
	ConnectContext,
	CreateConnStateContext,
	CreateContext,
	CreateVarsContext,
	DestroyContext,
	DisconnectContext,
	MigrateContext,
	RequestContext,
	RunContext,
	SleepContext,
	StateChangeContext,
	WakeContext,
	WebSocketContext,
} from "@/actor/config";
export type { ActorContextOf } from "@/actor/config";
import type { AnyActorDefinition, BaseActorDefinition } from "@/actor/definition";
import type { AnyDatabaseProvider } from "@/common/database/config";
import type { EventSchemaConfig, QueueSchemaConfig } from "@/actor/schema";

export type ActionContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? ActionContext<S, CP, CS, V, I, DB, E, Q>
		: never;

export type BeforeActionResponseContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? BeforeActionResponseContext<S, CP, CS, V, I, DB, E, Q>
		: never;

export type BeforeConnectContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		any,
		any,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? BeforeConnectContext<S, V, I, DB, E, Q>
		: never;

export type ConnectContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? ConnectContext<S, CP, CS, V, I, DB, E, Q>
		: never;

export type ConnContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? ActionContext<S, CP, CS, V, I, DB, E, Q>
		: never;

export type ConnInitContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		any,
		any,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? BeforeConnectContext<S, V, I, DB, E, Q>
		: never;

export type CreateConnStateContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		any,
		any,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? CreateConnStateContext<S, V, I, DB, E, Q>
		: never;

export type CreateContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		any,
		any,
		any,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? CreateContext<S, I, DB, E, Q>
		: never;

export type CreateVarsContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		any,
		any,
		any,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? CreateVarsContext<S, I, DB, E, Q>
		: never;

export type DestroyContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? DestroyContext<S, CP, CS, V, I, DB, E, Q>
		: never;

export type DisconnectContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? DisconnectContext<S, CP, CS, V, I, DB, E, Q>
		: never;

export type MigrateContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? MigrateContext<S, CP, CS, V, I, DB, E, Q>
		: never;

export type RequestContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? RequestContext<S, CP, CS, V, I, DB, E, Q>
		: never;

export type RunContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? RunContext<S, CP, CS, V, I, DB, E, Q>
		: never;

export type SleepContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? SleepContext<S, CP, CS, V, I, DB, E, Q>
		: never;

export type StateChangeContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? StateChangeContext<S, CP, CS, V, I, DB, E, Q>
		: never;

export type WakeContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? WakeContext<S, CP, CS, V, I, DB, E, Q>
		: never;

export type WebSocketContextOf<AD extends AnyActorDefinition> =
	AD extends BaseActorDefinition<
		infer S,
		infer CP,
		infer CS,
		infer V,
		infer I,
		infer DB extends AnyDatabaseProvider,
		infer E extends EventSchemaConfig,
		infer Q extends QueueSchemaConfig,
		any
	>
		? WebSocketContext<S, CP, CS, V, I, DB, E, Q>
		: never;
