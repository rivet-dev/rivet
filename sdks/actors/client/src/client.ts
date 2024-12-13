import type { ActorTags } from "../../common/src/utils.ts";
import type {
	ActorsRequest,
	ActorsResponse,
	RivetConfigResponse,
} from "../../manager-protocol/src/mod.ts";
import type { CreateRequest } from "../../manager-protocol/src/query.ts";
import type { ProtocolFormat } from "../../protocol/src/ws/mod.ts";
import type { AnyActor } from "../../runtime/src/actor.ts";
import * as errors from "./errors.ts";
import { ActorHandleRaw } from "./handle.ts";
import { logger } from "./log.ts";

export interface ClientOptions {
	protocolFormat?: ProtocolFormat;
}

export interface QueryOptions {
	/** Parameters to pass to the connection. */
	parameters?: unknown;
}

export interface GetWithIdOptions extends QueryOptions {}

export interface GetOptions extends QueryOptions {
	/** Prevents creating a new actor if one does not exist. */
	noCreate?: boolean;
	/** Config used to create the actor. */
	create?: Partial<CreateRequest>;
}

export interface CreateOptions extends QueryOptions {
	/** Config used to create the actor. */
	create: CreateRequest;
}

/**
 * Proxied wrapper of `RawActorHandle` that allows calling RPC functions
 * implicitly.
 *
 * Private methods (e.g. those starting with `_`) are automatically excluded.
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
 * RPC function returned by the actor proxy. This will call `ActorHandle.rpc`
 * when triggered.
 */
export type ActorRPCFunction<
	Args extends Array<unknown> = unknown[],
	Response = unknown,
> = (
	// Remove the first parameter, since that's `Context<...>`
	...args: Args extends [unknown, ...infer Rest] ? Rest : Args
) => Promise<Response>;

/** Region to connect to. */
interface Region {
	id: string;
	name: string;
}

export class Client {
	#managerEndpointPromise: Promise<string>;
	#regionPromise: Promise<Region | undefined>;
	#protocolFormat: ProtocolFormat;

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

	async getWithId<A extends AnyActor = AnyActor>(
		actorId: string,
		opts?: GetWithIdOptions,
	) {
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

	async get<A extends AnyActor = AnyActor>(
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

		logger().debug("get actor", { tags, parameters: opts?.parameters, create });

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

	async create<A extends AnyActor = AnyActor>(
		opts: CreateOptions,
	): Promise<ActorHandle<A>> {
		// Build create config
		const create = opts.create;

		// Default to the chosen region
		if (!create.region) create.region = (await this.#regionPromise)?.id;

		logger().debug("create actor", { parameters: opts?.parameters, create });

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
			get(target: ActorHandleRaw, prop: string | symbol, receiver: unknown) {
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
						method = (...args: unknown[]) => target.rpc(prop, ...args);
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
			getOwnPropertyDescriptor(target: ActorHandleRaw, prop: string | symbol) {
				const targetDescriptor = Reflect.getOwnPropertyDescriptor(target, prop);
				if (targetDescriptor) {
					return targetDescriptor;
				}
				if (typeof prop === "string") {
					// Make RPC methods appear non-enumerable
					return {
						configurable: true,
						enumerable: false,
						writable: false,
						value: (...args: unknown[]) => target.rpc(prop, ...args),
					};
				}
				return undefined;
			},
		}) as ActorHandle;
	}

	/** Sends an HTTP request to the manager actor. */
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
				throw new errors.ManagerError(`${res.statusText}: ${await res.text()}`);
			}

			return res.json();
		} catch (error) {
			throw new errors.ManagerError(String(error), { cause: error });
		}
	}

	async #fetchRegion(): Promise<Region | undefined> {
		try {
			// Fetch the connection info from the manager
			const { endpoint, project, environment } = await this.#sendManagerRequest<
				undefined,
				RivetConfigResponse
			>("GET", "/rivet/config");

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
				logger().error("failed to fetch region, defaulting to manager region", {
					status: res.statusText,
					body: await res.text(),
				});
				return undefined;
			}

			const { region }: { region: Region } = await res.json();

			return region;
		} catch (error) {
			// Add safe fallback in case we can't fetch the region
			logger().error("failed to fetch region, defaulting to manager region", {
				error,
			});
			return undefined;
		}
	}
}
