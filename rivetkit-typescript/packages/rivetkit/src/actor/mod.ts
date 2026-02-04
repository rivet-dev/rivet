import {
	type Actions,
	type ActorConfig,
	type ActorConfigInput,
	ActorConfigSchema,
	ActorTypes,
} from "./config";
import type { AnyDatabaseProvider } from "./database";
import { ActorDefinition } from "./definition";
import type { SchemaConfig } from "./schema";

export function actor<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TEvents extends SchemaConfig = Record<never, never>,
	TQueues extends SchemaConfig = Record<never, never>,
	TActions extends Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	> = Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>,
>(
	input: ActorConfigInput<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues,
		TActions
	>,
): ActorDefinition<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase,
	TEvents,
	TQueues,
	TActions
> {
	const config = ActorConfigSchema.parse(input) as ActorConfig<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TEvents,
		TQueues
	>;
	return new ActorDefinition(config);
}
export type { Encoding } from "@/actor/protocol/serde";
export {
	ALLOWED_PUBLIC_HEADERS,
	PATH_CONNECT,
	PATH_WEBSOCKET_PREFIX,
} from "@/common/actor-router-consts";
export type {
	UniversalErrorEvent,
	UniversalEvent,
	UniversalEventSource,
	UniversalMessageEvent,
} from "@/common/eventsource-interface";
export type {
	RivetCloseEvent,
	RivetEvent,
	RivetMessageEvent,
	UniversalWebSocket,
} from "@/common/websocket-interface";
export type { ActorKey } from "@/manager/protocol/query";
export type * from "./config";
export type { AnyConn, Conn } from "./conn/mod";
export type { ActorDefinition, AnyActorDefinition } from "./definition";
export { lookupInRegistry } from "./definition";
export { UserError, type UserErrorOptions } from "./errors";
export { KEYS as KV_KEYS } from "./instance/keys";
export { ActorKv } from "./instance/kv";
export type { AnyActorInstance } from "./instance/mod";
export {
	type ActorRouter,
	createActorRouter,
} from "./router";
export { routeWebSocket } from "./router-websocket-endpoints";
export { type Raw, raw } from "./schema";
