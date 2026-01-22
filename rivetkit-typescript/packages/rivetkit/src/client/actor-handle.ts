import * as cbor from "cbor-x";
import invariant from "invariant";
import type { AnyActorDefinition } from "@/actor/definition";
import type { Encoding } from "@/actor/protocol/serde";
import { assertUnreachable } from "@/actor/utils";
import { deconstructError } from "@/common/utils";
import {
	HEADER_CONN_PARAMS,
	HEADER_ENCODING,
	type ManagerDriver,
} from "@/driver-helpers/mod";
import type { ActorQuery } from "@/manager/protocol/query";
import type * as protocol from "@/schemas/client-protocol/mod";
import {
	CURRENT_VERSION as CLIENT_PROTOCOL_CURRENT_VERSION,
	HTTP_ACTION_REQUEST_VERSIONED,
	HTTP_ACTION_RESPONSE_VERSIONED,
} from "@/schemas/client-protocol/versioned";
import {
	type HttpActionRequest as HttpActionRequestJson,
	HttpActionRequestSchema,
	type HttpActionResponse as HttpActionResponseJson,
	HttpActionResponseSchema,
} from "@/schemas/client-protocol-zod/mod";
import { bufferToArrayBuffer } from "@/utils";
import type { ActorDefinitionActions } from "./actor-common";
import { type ActorConn, ActorConnRaw } from "./actor-conn";
import { checkForSchedulingError, queryActor } from "./actor-query";
import { type ClientRaw, CREATE_ACTOR_CONN_PROXY } from "./client";
import { ActorError, isSchedulingError } from "./errors";
import { logger } from "./log";
import { createQueueProxy, createQueueSender } from "./queue";
import { rawHttpFetch, rawWebSocket } from "./raw-utils";
import { sendHttpRequest } from "./utils";

/**
 * Provides underlying functions for stateless {@link ActorHandle} for action calls.
 * Similar to ActorConnRaw but doesn't maintain a connection.
 *
 * @see {@link ActorHandle}
 */
export class ActorHandleRaw {
	#client: ClientRaw;
	#driver: ManagerDriver;
	#encoding: Encoding;
	#actorQuery: ActorQuery;
	#params: unknown;
	#queueSender: ReturnType<typeof createQueueSender>;

	/**
	 * Do not call this directly.
	 *
	 * Creates an instance of ActorHandleRaw.
	 *
	 * @protected
	 */
	public constructor(
		client: any,
		driver: ManagerDriver,
		params: unknown,
		encoding: Encoding,
		actorQuery: ActorQuery,
	) {
		this.#client = client;
		this.#driver = driver;
		this.#encoding = encoding;
		this.#actorQuery = actorQuery;
		this.#params = params;
		this.#queueSender = createQueueSender({
			encoding: this.#encoding,
			params: this.#params,
			customFetch: async (request: Request) => {
				const { actorId } = await queryActor(
					undefined,
					this.#actorQuery,
					this.#driver,
				);
				return this.#driver.sendRequest(actorId, request);
			},
		});
	}

	get queue() {
		return createQueueProxy(this.#queueSender);
	}

	/**
	 * Call a raw action. This method sends an HTTP request to invoke the named action.
	 *
	 * @see {@link ActorHandle}
	 * @template Args - The type of arguments to pass to the action function.
	 * @template Response - The type of the response returned by the action function.
	 */
	async action<
		Args extends Array<unknown> = unknown[],
		Response = unknown,
	>(opts: {
		name: string;
		args: Args;
		signal?: AbortSignal;
	}): Promise<Response> {
		// Track actorId for scheduling error lookups
		let actorId: string | undefined;

		try {
			// Get the actor ID
			const result = await queryActor(
				undefined,
				this.#actorQuery,
				this.#driver,
			);
			actorId = result.actorId;
			logger().debug({ msg: "found actor for action", actorId });
			invariant(actorId, "Missing actor ID");

			// Invoke the action
			logger().debug({
				msg: "handling action",
				name: opts.name,
				encoding: this.#encoding,
			});
			const responseData = await sendHttpRequest<
				protocol.HttpActionRequest, // Bare type
				protocol.HttpActionResponse, // Bare type
				HttpActionRequestJson, // Json type
				HttpActionResponseJson, // Json type
				unknown[], // Request type (the args array)
				Response // Response type (the output value)
			>({
				url: `http://actor/action/${encodeURIComponent(opts.name)}`,
				method: "POST",
				headers: {
					[HEADER_ENCODING]: this.#encoding,
					...(this.#params !== undefined
						? { [HEADER_CONN_PARAMS]: JSON.stringify(this.#params) }
						: {}),
				},
				body: opts.args,
				encoding: this.#encoding,
				customFetch: this.#driver.sendRequest.bind(
					this.#driver,
					actorId,
				),
				signal: opts?.signal,
				requestVersion: CLIENT_PROTOCOL_CURRENT_VERSION,
				requestVersionedDataHandler: HTTP_ACTION_REQUEST_VERSIONED,
				responseVersion: CLIENT_PROTOCOL_CURRENT_VERSION,
				responseVersionedDataHandler: HTTP_ACTION_RESPONSE_VERSIONED,
				requestZodSchema: HttpActionRequestSchema,
				responseZodSchema: HttpActionResponseSchema,
				// JSON Request: args is the raw value
				requestToJson: (args): HttpActionRequestJson => ({
					args,
				}),
				// BARE Request: args needs to be CBOR-encoded
				requestToBare: (args): protocol.HttpActionRequest => ({
					args: bufferToArrayBuffer(cbor.encode(args)),
				}),
				// JSON Response: output is the raw value
				responseFromJson: (json): Response => json.output as Response,
				// BARE Response: output is ArrayBuffer that needs CBOR-decoding
				responseFromBare: (bare): Response =>
					cbor.decode(new Uint8Array(bare.output)) as Response,
			});

			return responseData;
		} catch (err) {
			// Standardize to ClientActorError instead of the native backend error
			const { group, code, message, metadata } = deconstructError(
				err,
				logger(),
				{},
				true,
			);

			// Check if this is a scheduling error and try to get more details
			if (actorId && isSchedulingError(group, code)) {
				const schedulingError = await checkForSchedulingError(
					group,
					code,
					actorId,
					this.#actorQuery,
					this.#driver,
				);
				if (schedulingError) {
					throw schedulingError;
				}
			}

			throw new ActorError(group, code, message, metadata);
		}
	}

	/**
	 * Establishes a persistent connection to the actor.
	 *
	 * @template AD The actor class that this connection is for.
	 * @returns {ActorConn<AD>} A connection to the actor.
	 */
	connect(): ActorConn<AnyActorDefinition> {
		logger().debug({
			msg: "establishing connection from handle",
			query: this.#actorQuery,
		});

		const conn = new ActorConnRaw(
			this.#client,
			this.#driver,
			this.#params,
			this.#encoding,
			this.#actorQuery,
		);

		return this.#client[CREATE_ACTOR_CONN_PROXY](
			conn,
		) as ActorConn<AnyActorDefinition>;
	}

	/**
	 * Fetches a resource from this actor via the /request endpoint. This is a
	 * convenience wrapper around the raw HTTP API.
	 */
	fetch(input: string | URL | Request, init?: RequestInit) {
		return rawHttpFetch(
			this.#driver,
			this.#actorQuery,
			this.#params,
			input,
			init,
		);
	}

	/**
	 * Opens a raw WebSocket connection to this actor.
	 */
	webSocket(path?: string, protocols?: string | string[]) {
		return rawWebSocket(
			this.#driver,
			this.#actorQuery,
			this.#params,
			path,
			protocols,
		);
	}

	/**
	 * Resolves the actor to get its unique actor ID.
	 */
	async resolve(): Promise<string> {
		if ("getForKey" in this.#actorQuery) {
			const name = this.#actorQuery.getForKey.name;

			// Query the actor to get the id
			const { actorId } = await queryActor(
				undefined,
				this.#actorQuery,
				this.#driver,
			);

			this.#actorQuery = { getForId: { actorId, name } };

			return actorId;
		} else if ("getOrCreateForKey" in this.#actorQuery) {
			const name = this.#actorQuery.getOrCreateForKey.name;

			// Query the actor to get the id (will create if doesn't exist)
			const { actorId } = await queryActor(
				undefined,
				this.#actorQuery,
				this.#driver,
			);

			this.#actorQuery = { getForId: { actorId, name } };

			return actorId;
		} else if ("getForId" in this.#actorQuery) {
			// Skip since it's already resolved
			return this.#actorQuery.getForId.actorId;
		} else if ("create" in this.#actorQuery) {
			// Cannot create a handle with this query
			invariant(false, "actorQuery cannot be create");
		} else {
			assertUnreachable(this.#actorQuery);
		}
	}

	/**
	 * Returns the raw URL for routing traffic to the actor.
	 */
	async getGatewayUrl(): Promise<string> {
		const { actorId } = await queryActor(
			undefined,
			this.#actorQuery,
			this.#driver,
		);
		return await this.#driver.buildGatewayUrl(actorId);
	}
}

/**
 * Stateless handle to a actor. Allows calling actor's remote procedure calls with inferred types
 * without establishing a persistent connection.
 *
 * @example
 * ```
 * const room = client.get<ChatRoom>(...etc...);
 * // This calls the action named `sendMessage` on the `ChatRoom` actor without a connection.
 * await room.sendMessage('Hello, world!');
 * ```
 *
 * Private methods (e.g. those starting with `_`) are automatically excluded.
 *
 * @template AD The actor class that this handle is for.
 * @see {@link ActorHandleRaw}
 */
export type ActorHandle<AD extends AnyActorDefinition> = Omit<
	ActorHandleRaw,
	"connect"
> & {
	// Add typed version of ActorConn (instead of using AnyActorDefinition)
	connect(): ActorConn<AD>;
	// Resolve method returns the actor ID
	resolve(): Promise<string>;
} & ActorDefinitionActions<AD>;
