import { injectDevtools } from "@/devtools-loader";
import { RemoteEngineControlClient } from "@/engine-client/mod";
import type { Registry } from "@/registry";
import {
	type Client,
	type ClientConfigInput,
	createClientWithDriver,
} from "./client";
import { ClientConfigSchema } from "./config";

export type { ActorDefinition, AnyActorDefinition } from "@/actor/definition";
export {
	ActorClientError,
	ActorConnDisposed,
	ActorError,
	MalformedResponseMessage,
	ManagerError,
	RivetError,
	UserError,
} from "@/client/errors";
export type { CreateRequest } from "@/client/query";
export type { Encoding } from "@/common/encoding";
export type {
	ActorActionFunction,
	ActorActionOptions,
	ActorConnectOptions,
	ActorGatewayOptions,
} from "./actor-common";
export type {
	ActorConn,
	ActorConnStatus,
	ConnectionStateCallback,
	EventUnsubscribe,
	StatusChangeCallback,
} from "./actor-conn";
export { ActorConnRaw } from "./actor-conn";
export type { ActorHandle } from "./actor-handle";
export { ActorHandleRaw } from "./actor-handle";
export type {
	ActorAccessor,
	Client,
	ClientConfigInput,
	ClientRaw,
	CreateOptions,
	ExtractActorsFromRegistry,
	ExtractRegistryFromClient,
	GetOptions,
	GetWithIdOptions,
	QueryOptions,
	Region,
} from "./client";
export type { GetToken } from "./token-provider";

/**
 * Creates a client with the actor accessor proxy.
 */
export function createClient<A extends Registry<any>>(
	endpointOrConfig?: string | ClientConfigInput,
): Client<A> {
	// Parse config
	const configInput =
		endpointOrConfig === undefined
			? {}
			: typeof endpointOrConfig === "string"
				? { endpoint: endpointOrConfig }
				: endpointOrConfig;
	const config = ClientConfigSchema.parse(configInput);

	// Create client
	const driver = new RemoteEngineControlClient(config);

	if (config.devtools) {
		injectDevtools(config);
	}

	return createClientWithDriver<A>(driver, config);
}
