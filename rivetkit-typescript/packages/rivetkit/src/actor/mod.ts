import {
	type Actions,
	type ActorConfig,
	type ActorConfigInput,
	ActorConfigSchema,
	ActorTypes,
} from "./config";
import type { AnyDatabaseProvider } from "./database";
import { ActorDefinition } from "./definition";

export function actor<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase extends AnyDatabaseProvider,
	TActions extends Actions<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase
	>,
>(
	input: ActorConfigInput<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase,
		TActions
	>,
): ActorDefinition<
	TState,
	TConnParams,
	TConnState,
	TVars,
	TInput,
	TDatabase,
	TActions
> {
	const config = ActorConfigSchema.parse(input) as ActorConfig<
		TState,
		TConnParams,
		TConnState,
		TVars,
		TInput,
		TDatabase
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
export type { AnyActorInstance } from "./instance/mod";
export {
	type ActorRouter,
	createActorRouter,
} from "./router";
export { routeWebSocket } from "./router-websocket-endpoints";
