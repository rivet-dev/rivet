import {
	type Actions,
	type ActorConfig,
	type ActorConfigInput,
	ActorConfigSchema,
	ActorTypes,
	action,
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
export { action, handler } from "./config";
export type * from "./config";
export type { AnyConn, Conn } from "./conn/mod";
export type { ActionContext } from "./contexts/action";
export type { ActorContext } from "./contexts/actor";
export type { ConnInitContext } from "./contexts/conn-init";
export type { CreateConnStateContext } from "./contexts/create-conn-state";
export type { OnBeforeConnectContext } from "./contexts/on-before-connect";
export type { OnConnectContext } from "./contexts/on-connect";
export type { RequestContext } from "./contexts/request";
export type { WebSocketContext } from "./contexts/websocket";
export type {
	ActionContextOf,
	ActorContextOf,
	ActorDefinition,
	AnyActorDefinition,
} from "./definition";
export { lookupInRegistry } from "./definition";
export { UserError, type UserErrorOptions } from "./errors";
export type { AnyActorInstance } from "./instance/mod";
export {
	type ActorRouter,
	createActorRouter,
} from "./router";
export { routeWebSocket } from "./router-websocket-endpoints";
export type { AnyDatabaseProvider } from "./database";
