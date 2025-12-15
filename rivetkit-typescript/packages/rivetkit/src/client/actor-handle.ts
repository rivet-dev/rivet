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
import { queryActor } from "./actor-query";
import { type ClientRaw, CREATE_ACTOR_CONN_PROXY } from "./client";
import { ActorError } from "./errors";
import { logger } from "./log";
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
		// return await this.#driver.action<Args, Response>(
		// 	undefined,
		// 	this.#actorQuery,
		// 	this.#encodingKind,
		// 	this.#params,
		// 	opts.name,
		// 	opts.args,
		// 	{ signal: opts.signal },
		// );
		try {
			// Get the actor ID
			const { actorId } = await queryActor(
				undefined,
				this.#actorQuery,
				this.#driver,
			);
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
	 * Makes a raw HTTP request to the actor.
	 *
	 * @param input - The URL, path, or Request object
	 * @param init - Standard fetch RequestInit options
	 * @returns Promise<Response> - The raw HTTP response
	 */
	async fetch(
		input: string | URL | Request,
		init?: RequestInit,
	): Promise<Response> {
		return rawHttpFetch(
			this.#driver,
			this.#actorQuery,
			this.#params,
			input,
			init,
		);
	}

	/**
	 * Creates a raw WebSocket connection to the actor.
	 *
	 * @param path - The path for the WebSocket connection (e.g., "stream")
	 * @param protocols - Optional WebSocket subprotocols
	 * @returns WebSocket - A raw WebSocket connection
	 */
	async websocket(
		path?: string,
		protocols?: string | string[],
	): Promise<WebSocket> {
		return rawWebSocket(
			this.#driver,
			this.#actorQuery,
			this.#params,
			path,
			protocols,
		);
	}

	/**
	 * Resolves the actor to get its unique actor ID
	 *
	 * @returns {Promise<string>} - A promise that resolves to the actor's ID
	 */
	async resolve({ signal }: { signal?: AbortSignal } = {}): Promise<string> {
		if (
			"getForKey" in this.#actorQuery ||
			"getOrCreateForKey" in this.#actorQuery
		) {
			// TODO:
			let name: string;
			if ("getForKey" in this.#actorQuery) {
				name = this.#actorQuery.getForKey.name;
			} else if ("getOrCreateForKey" in this.#actorQuery) {
				name = this.#actorQuery.getOrCreateForKey.name;
			} else {
				assertUnreachable(this.#actorQuery);
			}

			const { actorId } = await queryActor(
				undefined,
				this.#actorQuery,
				this.#driver,
			);

			this.#actorQuery = { getForId: { actorId, name } };

			return actorId;
		} else if ("getForId" in this.#actorQuery) {
			// SKip since it's already resolved
			return this.#actorQuery.getForId.actorId;
		} else if ("create" in this.#actorQuery) {
			// Cannot create a handle with this query
			invariant(false, "actorQuery cannot be create");
		} else {
			assertUnreachable(this.#actorQuery);
		}
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
