import * as cbor from "cbor-x";
import type { AnyActorDefinition } from "@/actor/definition";
import type { Encoding } from "@/common/encoding";
import {
	HEADER_CONN_PARAMS,
	HEADER_ENCODING,
} from "@/common/actor-router-consts";
import type * as protocol from "@/common/client-protocol";
import {
	CURRENT_VERSION as CLIENT_PROTOCOL_CURRENT_VERSION,
	HTTP_ACTION_REQUEST_VERSIONED,
	HTTP_ACTION_RESPONSE_VERSIONED,
} from "@/common/client-protocol-versioned";
import {
	type HttpActionRequest as HttpActionRequestJson,
	HttpActionRequestSchema,
	type HttpActionResponse as HttpActionResponseJson,
	HttpActionResponseSchema,
} from "@/common/client-protocol-zod";
import { deconstructError } from "@/common/utils";
import type { EngineControlClient } from "@/engine-client/driver";
import { bufferToArrayBuffer } from "@/utils";
import type {
	ActorDefinitionActions,
	ActorDefinitionQueueSend,
} from "./actor-common";
import { type ActorConn, ActorConnRaw } from "./actor-conn";
import {
	type ActorResolutionState,
	checkForSchedulingError,
	getGatewayTarget,
	isDynamicActorQuery,
	isStaleResolvedActorError,
} from "./actor-query";
import { type ClientRaw, CREATE_ACTOR_CONN_PROXY } from "./client";
import { ActorError, isSchedulingError } from "./errors";
import { logger } from "./log";
import {
	createQueueSender,
	type QueueSendNoWaitOptions,
	type QueueSendOptions,
	type QueueSendResult,
	type QueueSendWaitOptions,
} from "./queue";
import { rawHttpFetch, rawWebSocket } from "./raw-utils";
import { resolveGatewayTarget } from "./resolve-gateway-target";
import { sendHttpRequest } from "./utils";

/**
 * Provides underlying functions for stateless {@link ActorHandle} for action calls.
 * Similar to ActorConnRaw but doesn't maintain a connection.
 *
 * @see {@link ActorHandle}
 */
export class ActorHandleRaw {
	#client: ClientRaw;
	#driver: EngineControlClient;
	#encoding: Encoding;
	#actorResolutionState: ActorResolutionState;
	#params: unknown;
	#getParams?: () => Promise<unknown>;
	#queueSender: ReturnType<typeof createQueueSender>;
	#resolvedActorId?: string;
	#resolvingActorId?: Promise<string>;

	/**
	 * Do not call this directly.
	 *
	 * Creates an instance of ActorHandleRaw.
	 *
	 * @protected
	 */
	public constructor(
		client: any,
		driver: EngineControlClient,
		params: unknown,
		getParams: (() => Promise<unknown>) | undefined,
		encoding: Encoding,
		actorResolutionState: ActorResolutionState,
	) {
		this.#client = client;
		this.#driver = driver;
		this.#encoding = encoding;
		this.#actorResolutionState = actorResolutionState;
		this.#params = params;
		this.#getParams = getParams;
		// Resolve the actor ID for each queue send so key-based handles do not
		// pin themselves to an earlier resolution.
		this.#queueSender = createQueueSender({
			encoding: this.#encoding,
			params: this.#params,
			customFetch: async (request: Request) => {
				return await this.#driver.sendRequest(
					getGatewayTarget(this.#actorResolutionState),
					request,
				);
			},
		});
	}

	async #resolveConnectionParams(): Promise<unknown> {
		if (this.#getParams) {
			return await this.#getParams();
		}

		return this.#params;
	}

	send(
		name: string,
		body: unknown,
		options: QueueSendWaitOptions,
	): Promise<QueueSendResult>;
	send(
		name: string,
		body: unknown,
		options?: QueueSendNoWaitOptions,
	): Promise<void>;
	send(
		name: string,
		body: unknown,
		options?: QueueSendOptions,
	): Promise<QueueSendResult | void> {
		return this.#sendQueueMessage(name, body, options as any);
	}

	async #sendQueueMessage(
		name: string,
		body: unknown,
		options?: QueueSendOptions,
	): Promise<QueueSendResult | void> {
		return await this.#queueSender.send(name, body, options as any);
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
		if (
			typeof opts === "string" ||
			typeof opts !== "object" ||
			opts === null ||
			!("name" in opts)
		) {
			throw new Error(
				`Invalid action call: expected an options object { name, args }, got ${typeof opts}. Use handle.actionName(...args) for the shorthand API.`,
			);
		}
		return (await this.#sendActionNow(opts)) as Response;
	}

	async #sendActionNow(opts: {
		name: string;
		args: unknown[];
		signal?: AbortSignal;
	}): Promise<unknown> {
		const maxAttempts = isDynamicActorQuery(this.#actorResolutionState) ? 2 : 1;

		for (let attempt = 0; attempt < maxAttempts; attempt++) {
			let actorId: string | undefined;
			try {
				const target = await this.#resolveActionTarget();
				actorId = "directId" in target ? target.directId : undefined;

				logger().debug(
					actorId
						? { msg: "using direct actor gateway target", actorId }
						: {
								msg: "using query gateway target for action",
								query: this.#actorResolutionState,
							},
				);

				logger().debug({
					msg: "handling action",
					name: opts.name,
					encoding: this.#encoding,
				});
				return await sendHttpRequest<
					protocol.HttpActionRequest,
					protocol.HttpActionResponse,
					HttpActionRequestJson,
					HttpActionResponseJson,
					unknown[],
					Response
				>({
					url: `http://actor/action/${encodeURIComponent(opts.name)}`,
					method: "POST",
					headers: {
						[HEADER_ENCODING]: this.#encoding,
						...(this.#params !== undefined
							? {
									[HEADER_CONN_PARAMS]: JSON.stringify(
										this.#params,
									),
								}
							: {}),
					},
					body: opts.args,
					encoding: this.#encoding,
					customFetch: this.#driver.sendRequest.bind(
						this.#driver,
						target,
					),
					signal: opts?.signal,
					requestVersion: CLIENT_PROTOCOL_CURRENT_VERSION,
					requestVersionedDataHandler: HTTP_ACTION_REQUEST_VERSIONED,
					responseVersion: CLIENT_PROTOCOL_CURRENT_VERSION,
					responseVersionedDataHandler: HTTP_ACTION_RESPONSE_VERSIONED,
					requestZodSchema: HttpActionRequestSchema,
					responseZodSchema: HttpActionResponseSchema,
					requestToJson: (args): HttpActionRequestJson => ({
						args,
					}),
					requestToBare: (args): protocol.HttpActionRequest => ({
						args: bufferToArrayBuffer(cbor.encode(args)),
					}),
					responseFromJson: (json): Response => json.output as Response,
					responseFromBare: (bare): Response =>
						cbor.decode(new Uint8Array(bare.output)) as Response,
				});
			} catch (err) {
				const { group, code, message, metadata } = deconstructError(
					err,
					logger(),
					{},
					true,
				);

				if (actorId && isSchedulingError(group, code)) {
					const schedulingError = await checkForSchedulingError(
						group,
						code,
						actorId,
						this.#actorResolutionState,
						this.#driver,
					);
					if (schedulingError) {
						throw schedulingError;
					}
				}

				if (
					group === "actor" &&
					code === "destroyed_while_waiting_for_ready" &&
					"getForId" in this.#actorResolutionState
				) {
					throw new ActorError(
						"actor",
						"not_found",
						"The actor does not exist or was destroyed.",
						metadata,
					);
				}

				const invalidated = this.#invalidateResolvedActorId(group, code);
				if (invalidated && attempt < maxAttempts - 1) {
					continue;
				}

				throw new ActorError(group, code, message, metadata);
			}
		}

		throw new Error("unreachable action retry state");
	}

	#clearResolvedActorId(): void {
		this.#resolvedActorId = undefined;
		this.#resolvingActorId = undefined;
	}

	#invalidateResolvedActorId(group: string, code: string): boolean {
		if (
			!isDynamicActorQuery(this.#actorResolutionState) ||
			!isStaleResolvedActorError(group, code)
		) {
			return false;
		}

		this.#clearResolvedActorId();
		return true;
	}

	async #resolveActionTarget() {
		if ("getForId" in this.#actorResolutionState) {
			return getGatewayTarget(this.#actorResolutionState);
		}

		if (this.#resolvedActorId) {
			return { directId: this.#resolvedActorId } as const;
		}

		if (!this.#resolvingActorId) {
			this.#resolvingActorId = resolveGatewayTarget(
				this.#driver,
				this.#actorResolutionState,
			).then((actorId) => {
				this.#resolvedActorId = actorId;
				return actorId;
			});
		}

		try {
			return { directId: await this.#resolvingActorId } as const;
		} finally {
			this.#resolvingActorId = undefined;
		}
	}

	/**
	 * Establishes a persistent connection to the actor.
	 *
	 * @template AD The actor class that this connection is for.
	 * @returns {ActorConn<AD>} A connection to the actor.
	 */
	connect(params?: unknown): ActorConn<AnyActorDefinition> {
		logger().debug({
			msg: "establishing connection from handle",
			query: this.#actorResolutionState,
		});

		const connParams = params === undefined ? this.#params : params;
		const getParams = params === undefined ? this.#getParams : undefined;

		const conn = new ActorConnRaw(
			this.#client,
			this.#driver,
			connParams,
			getParams,
			this.#encoding,
			this.#actorResolutionState,
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
		return this.#fetchWithResolvedActor(input, init);
	}

	async #fetchWithResolvedActor(
		input: string | URL | Request,
		init?: RequestInit,
	) {
		return await rawHttpFetch(
			this.#driver,
			getGatewayTarget(this.#actorResolutionState),
			this.#params,
			input,
			init,
		);
	}

	/**
	 * Opens a raw WebSocket connection to this actor.
	 */
	async webSocket(path?: string, protocols?: string | string[]) {
		const params = await this.#resolveConnectionParams();
		return await rawWebSocket(
			this.#driver,
			getGatewayTarget(this.#actorResolutionState),
			params,
			path,
			protocols,
		);
	}

	/**
	 * Resolves the actor to get its unique actor ID.
	 */
	async resolve(): Promise<string> {
		if ("getForId" in this.#actorResolutionState) {
			return this.#actorResolutionState.getForId.actorId;
		}

		const target = await this.#resolveActionTarget();
		if ("directId" in target) {
			return target.directId;
		}

		throw new Error("dynamic actor resolution did not produce a direct actor id");
	}

	/**
	 * Returns the raw URL for routing traffic to the actor.
	 */
	async getGatewayUrl(): Promise<string> {
		return await this.#driver.buildGatewayUrl(
			getGatewayTarget(this.#actorResolutionState),
		);
	}

	async reload(): Promise<void> {
		const target = getGatewayTarget(this.#actorResolutionState);
		const request = new Request("http://actor/dynamic/reload", {
			method: "PUT",
		});
		const response = await this.#driver.sendRequest(target, request);
		if (!response.ok) {
			const body = await response.text().catch(() => "");
			throw new ActorError(
				"actor",
				"reload_failed",
				`reload failed with status ${response.status}: ${body}`,
				{},
			);
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
	"connect" | "send"
> & {
	// Add typed version of ActorConn (instead of using AnyActorDefinition)
	connect(params?: unknown): ActorConn<AD>;
	// Resolve method returns the actor ID
	resolve(): Promise<string>;
} & ActorDefinitionQueueSend<AD> &
	ActorDefinitionActions<AD>;
