import type { Registry } from "@/registry/mod";
import { RemoteManagerDriver } from "@/remote-manager-driver/mod";
import {
	type Client,
	type ClientConfigInput,
	createClientWithDriver,
} from "./client";
import { ClientConfigSchema } from "./config";

export {
	ActorDefinition,
	AnyActorDefinition,
} from "@/actor/definition";
export type { Encoding } from "@/actor/protocol/serde";
export {
	ActorClientError,
	ActorError,
	InternalError,
	MalformedResponseMessage,
	ManagerError,
} from "@/client/errors";
export type { CreateRequest } from "@/manager/protocol/query";
export type { ActorActionFunction } from "./actor-common";
export type {
	ActorConn,
	ConnectionStateCallback,
	EventUnsubscribe,
} from "./actor-conn";
export { ActorConnRaw } from "./actor-conn";
export type { ActorHandle } from "./actor-handle";
export { ActorHandleRaw } from "./actor-handle";
export type {
	ActorAccessor,
	Client,
	ClientRaw,
	CreateOptions,
	ExtractActorsFromRegistry,
	ExtractRegistryFromClient,
	GetOptions,
	GetWithIdOptions,
	QueryOptions,
	Region,
} from "./client";

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
	const driver = new RemoteManagerDriver(config);
	return createClientWithDriver<A>(driver, config);
}
