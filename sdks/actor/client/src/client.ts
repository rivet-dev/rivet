import type { ActorTags } from "@rivet-gg/actor-common/utils";
import type { ProtocolFormat } from "@rivet-gg/actor-protocol/ws";
import type {
	ActorsRequest,
	ActorsResponse,
	RivetConfigResponse,
} from "@rivet-gg/manager-protocol";
import type { CreateRequest } from "@rivet-gg/manager-protocol/query";
import * as errors from "./errors";
import { ActorHandleRaw } from "./handle";
import { logger } from "./log";

/**
 * Options for configuring the client.
 * @typedef {Object} ClientOptions
 * @property {ProtocolFormat} [protocolFormat] - The format used for protocol communication.
 */
export interface ClientOptions {
	protocolFormat?: ProtocolFormat;
}

/**
 * Options for querying actors.
 * @typedef {Object} QueryOptions
 * @property {unknown} [parameters] - Parameters to pass to the connection.
 */
export interface QueryOptions {
	/** Parameters to pass to the connection. */
	parameters?: unknown;
}

/**
 * Options for getting an actor by ID.
 * @typedef {QueryOptions} GetWithIdOptions
 */
export interface GetWithIdOptions extends QueryOptions {}

/**
 * Options for getting an actor.
 * @typedef {QueryOptions} GetOptions
 * @property {boolean} [noCreate] - Prevents creating a new actor if one does not exist.
 * @property {Partial<CreateRequest>} [create] - Config used to create the actor.
 */
export interface GetOptions extends QueryOptions {
	/** Prevents creating a new actor if one does not exist. */
	noCreate?: boolean;
	/** Config used to create the actor. */
	create?: Partial<CreateRequest>;
}

/**
 * Options for creating an actor.
 * @typedef {QueryOptions} CreateOptions
 * @property {CreateRequest} create - Config used to create the actor.
 */
export interface CreateOptions extends QueryOptions {
	/** Config used to create the actor. */
	create: CreateRequest;
}

/**
 * Connection to an actor. Allows calling actor's remote procedure calls with inferred types. See {@link ActorHandleRaw} for underlying methods.
 *
 * @example
 * ```
 * const room = await client.get<ChatRoom>(...etc...);
 * // This calls the rpc named `sendMessage` on the `ChatRoom` actor.
 * await room.sendMessage('Hello, world!');
 * ```
 *
 * Private methods (e.g. those starting with `_`) are automatically excluded.
 *
 * @template A The actor class that this handle is connected to.
 * @see {@link ActorHandleRaw}
 */
export type ActorHandle<A = unknown> = ActorHandleRaw & {
	[K in keyof A as K extends string
		? K extends `_${string}`
			? never
			: K
		: K]: A[K] extends (...args: infer Args) => infer Return
		? ActorRPCFunction<Args, Return>
		: never;
};

/**
 * RPC function returned by `ActorHandle`. This will call `ActorHandle.rpc` when triggered.
 *
 * @typedef {Function} ActorRPCFunction
 * @template Args
 * @template Response
 * @param {...Args} args - Arguments for the RPC function.
 * @returns {Promise<Response>}
 */
export type ActorRPCFunction<
	Args extends Array<unknown> = unknown[],
	Response = unknown,
> = (
	...args: Args extends [unknown, ...infer Rest] ? Rest : Args
) => Promise<Response>;

/**
 * Represents a region to connect to.
 * @typedef {Object} Region
 * @property {string} id - The region ID.
 * @property {string} name - The region name.
 * @see {@link https://rivet.gg/docs/edge|Edge Networking}
 * @see {@link https://rivet.gg/docs/regions|Available Regions}
 */
export interface Region {
	/**
	 * The region slug.
	 */
	id: string;

	/**
	 * The human-friendly region name.
	 */
	name: string;
}

/**
 * Client for managing & connecting to actors.
 * @see {@link https://rivet.gg/docs/manage|Create & Manage Actors}
 */
export class Client {
	#managerEndpointPromise: Promise<string>;
	#regionPromise: Promise<Region | undefined>;
	#protocolFormat: ProtocolFormat;

	/**
	 * Creates an instance of Client.
	 *
	 * @param {string | Promise<string>} managerEndpointPromise - The manager endpoint or a promise resolving to it. See {@link https://rivet.gg/docs/setup|Initial Setup} for instructions on getting the manager endpoint.
	 * @param {ClientOptions} [opts] - Options for configuring the client.
	 * @see {@link https://rivet.gg/docs/setup|Initial Setup}
	 */
	public constructor(
		managerEndpointPromise: string | Promise<string>,
		opts?: ClientOptions,
	) {
		if (managerEndpointPromise instanceof Promise) {
			// Save promise
			this.#managerEndpointPromise = managerEndpointPromise;
		} else {
			// Convert to promise
			this.#managerEndpointPromise = new Promise((resolve) =>
				resolve(managerEndpointPromise),
			);
		}

		this.#regionPromise = this.#fetchRegion();

		this.#protocolFormat = opts?.protocolFormat ?? "cbor";
	}

	/**
	 * Gets an actor by its ID.
	 * @template A The actor class that this handle is connected to.
	 * @param {string} actorId - The ID of the actor.
	 * @param {GetWithIdOptions} [opts] - Options for getting the actor.
	 * @returns {Promise<ActorHandle<A>>} - A promise resolving to the actor handle.
	 */
	async getWithId<A = unknown>(
		actorId: string,
		opts?: GetWithIdOptions,
	): Promise<ActorHandle<A>> {
		logger().debug("get actor with id ", {
			actorId,
			parameters: opts?.parameters,
		});

		const resJson = await this.#sendManagerRequest<
			ActorsRequest,
			ActorsResponse
		>("POST", "/actors", {
			query: {
				getForId: {
					actorId,
				},
			},
		});

		const handle = this.#createHandle(resJson.endpoint, opts?.parameters);
		return this.#createProxy(handle) as ActorHandle<A>;
	}

	/**
	 * Gets an actor by its tags, creating it if necessary.
	 *
	 * @example
	 * ```
	 * const room = await client.get<ChatRoom>({
	 *   name: 'chat_room',
	 *   // Get or create the actor for the channel `random`
	 *   channel: 'random'
	 * });
	 *
	 * // This actor will have the tags: { name: 'chat_room', channel: 'random' }
	 * await room.sendMessage('Hello, world!');
	 * ```
	 *
	 * @template A The actor class that this handle is connected to.
	 * @param {ActorTags} tags - The tags to identify the actor.
	 * @param {GetOptions} [opts] - Options for getting the actor.
	 * @returns {Promise<ActorHandle<A>>} - A promise resolving to the actor handle.
	 * @see {@link https://rivet.gg/docs/manage#client.get}
	 */
	async get<A = unknown>(
		tags: ActorTags,
		opts?: GetOptions,
	): Promise<ActorHandle<A>> {
		if (!("name" in tags)) throw new Error("Tags must contain name");

		// Build create config
		let create: CreateRequest | undefined = undefined;
		if (!opts?.noCreate) {
			create = {
				// Default to the same tags as the request
				tags: opts?.create?.tags ?? tags,
				// Default to the chosen region
				region: opts?.create?.region ?? (await this.#regionPromise)?.id,
			};
		}

		logger().debug("get actor", {
			tags,
			parameters: opts?.parameters,
			create,
		});

		const resJson = await this.#sendManagerRequest<
			ActorsRequest,
			ActorsResponse
		>("POST", "/actors", {
			query: {
				getOrCreateForTags: {
					tags,
					create,
				},
			},
		});

		const handle = this.#createHandle(resJson.endpoint, opts?.parameters);
		return this.#createProxy(handle) as ActorHandle<A>;
	}

	/**
	 * Creates a new actor with the provided tags.
	 *
	 * @example
	 * ```
	 * // Create a new document actor
	 * const doc = await client.create<MyDocument>({
	 *   create: {
	 *     tags: {
	 *       name: 'my_document',
	 *       docId: '123'
	 *     }
	 *   }
	 * });
	 *
	 * await doc.doSomething();
	 * ```
	 *
	 * @template A The actor class that this handle is connected to.
	 * @param {CreateOptions} opts - Options for creating the actor.
	 * @returns {Promise<ActorHandle<A>>} - A promise resolving to the actor handle.
	 * @see {@link https://rivet.gg/docs/manage#client.create}
	 */
	async create<A = unknown>(opts: CreateOptions): Promise<ActorHandle<A>> {
		// Build create config
		const create = opts.create;

		// Default to the chosen region
		if (!create.region) create.region = (await this.#regionPromise)?.id;

		logger().debug("create actor", {
			parameters: opts?.parameters,
			create,
		});

		const resJson = await this.#sendManagerRequest<
			ActorsRequest,
			ActorsResponse
		>("POST", "/actors", {
			query: {
				create,
			},
		});

		const handle = this.#createHandle(resJson.endpoint, opts?.parameters);
		return this.#createProxy(handle) as ActorHandle<A>;
	}

	#createHandle(endpoint: string, parameters: unknown): ActorHandleRaw {
		const handle = new ActorHandleRaw(
			endpoint,
			parameters,
			this.#protocolFormat,
		);
		handle.connect();
		return handle;
	}

	#createProxy(handle: ActorHandleRaw): ActorHandle {
		// Stores returned RPC functions for faster calls
		const methodCache = new Map<string, ActorRPCFunction>();
		return new Proxy(handle, {
			get(
				target: ActorHandleRaw,
				prop: string | symbol,
				receiver: unknown,
			) {
				// Handle built-in Symbol properties
				if (typeof prop === "symbol") {
					return Reflect.get(target, prop, receiver);
				}

				// Handle built-in Promise methods and existing properties
				if (
					prop === "then" ||
					prop === "catch" ||
					prop === "finally" ||
					prop === "constructor" ||
					prop in target
				) {
					const value = Reflect.get(target, prop, receiver);
					// Preserve method binding
					if (typeof value === "function") {
						return value.bind(target);
					}
					return value;
				}

				// Create RPC function that preserves 'this' context
				if (typeof prop === "string") {
					let method = methodCache.get(prop);
					if (!method) {
						method = (...args: unknown[]) =>
							target.rpc(prop, ...args);
						methodCache.set(prop, method);
					}
					return method;
				}
			},

			// Support for 'in' operator
			has(target: ActorHandleRaw, prop: string | symbol) {
				// All string properties are potentially RPC functions
				if (typeof prop === "string") {
					return true;
				}
				// For symbols, defer to the target's own has behavior
				return Reflect.has(target, prop);
			},

			// Support instanceof checks
			getPrototypeOf(target: ActorHandleRaw) {
				return Reflect.getPrototypeOf(target);
			},

			// Prevent property enumeration of non-existent RPC methods
			ownKeys(target: ActorHandleRaw) {
				return Reflect.ownKeys(target);
			},

			// Support proper property descriptors
			getOwnPropertyDescriptor(
				target: ActorHandleRaw,
				prop: string | symbol,
			) {
				const targetDescriptor = Reflect.getOwnPropertyDescriptor(
					target,
					prop,
				);
				if (targetDescriptor) {
					return targetDescriptor;
				}
				if (typeof prop === "string") {
					// Make RPC methods appear non-enumerable
					return {
						configurable: true,
						enumerable: false,
						writable: false,
						value: (...args: unknown[]) =>
							target.rpc(prop, ...args),
					};
				}
				return undefined;
			},
		}) as ActorHandle;
	}

	/**
	 * Sends an HTTP request to the manager actor.
	 * @private
	 * @template Request
	 * @template Response
	 * @param {string} method - The HTTP method.
	 * @param {string} path - The path for the request.
	 * @param {Request} [body] - The request body.
	 * @returns {Promise<Response>} - A promise resolving to the response.
	 * @see {@link https://rivet.gg/docs/manage#client}
	 */
	async #sendManagerRequest<Request, Response>(
		method: string,
		path: string,
		body?: Request,
	): Promise<Response> {
		try {
			const managerEndpoint = await this.#managerEndpointPromise;
			const res = await fetch(`${managerEndpoint}${path}`, {
				method,
				headers: {
					"Content-Type": "application/json",
				},
				body: body ? JSON.stringify(body) : undefined,
			});

			if (!res.ok) {
				throw new errors.ManagerError(
					`${res.statusText}: ${await res.text()}`,
				);
			}

			return res.json();
		} catch (error) {
			throw new errors.ManagerError(String(error), { cause: error });
		}
	}

	/**
	 * Fetches the region information.
	 * @private
	 * @returns {Promise<Region | undefined>} - A promise resolving to the region or undefined.
	 * @see {@link https://rivet.gg/docs/edge#Fetching-regions-via-API}
	 */
	async #fetchRegion(): Promise<Region | undefined> {
		try {
			// Fetch the connection info from the manager
			const { endpoint, project, environment } =
				await this.#sendManagerRequest<undefined, RivetConfigResponse>(
					"GET",
					"/rivet/config",
				);

			// Fetch the region
			//
			// This is fetched from the client instead of the manager so Rivet
			// can automatically determine the recommended region using an
			// anycast request made from the client
			const url = new URL("/regions/resolve", endpoint);
			if (project) url.searchParams.set("project", project);
			if (environment) url.searchParams.set("environment", environment);
			const res = await fetch(url.toString());

			if (!res.ok) {
				// Add safe fallback in case we can't fetch the region
				logger().error(
					"failed to fetch region, defaulting to manager region",
					{
						status: res.statusText,
						body: await res.text(),
					},
				);
				return undefined;
			}

			const { region }: { region: Region } = await res.json();

			return region;
		} catch (error) {
			// Add safe fallback in case we can't fetch the region
			logger().error(
				"failed to fetch region, defaulting to manager region",
				{
					error,
				},
			);
			return undefined;
		}
	}
}
