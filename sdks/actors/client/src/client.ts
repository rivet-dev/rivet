import type { ActorTags } from "../../common/src/utils.ts";
import type {
	ActorsRequest,
	ActorsResponse,
	RivetConfigResponse,
} from "../../manager-protocol/src/mod.ts";
import type { CreateRequest } from "../../manager-protocol/src/query.ts";
import { ActorHandleRaw } from "./handle.ts";
import { logger } from "./log.ts";

export interface GetOpts {
	parameters?: unknown;
	create?: CreateRequest;
}

/**
 * Proxied wrapper of `RawActorHandle` that allows calling RPC functions
 * implicitly.
 *
 * Private methods (e.g. those starting with `_`) are automatically excluded.
 */
export type ActorHandle<A = unknown> =
	& ActorHandleRaw
	& {
		[
			K in keyof A as K extends string ? K extends `_${string}` ? never
				: K
				: K
		]: A[K] extends (...args: infer Args) => infer Return
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

	constructor(managerEndpointPromise: string | Promise<string>) {
		if (managerEndpointPromise instanceof Promise) {
			// Save promise
			this.#managerEndpointPromise = managerEndpointPromise;
		} else {
			// Convert to promise
			this.#managerEndpointPromise = new Promise((resolve) =>
				resolve(managerEndpointPromise)
			);
		}

		this.#regionPromise = this.#fetchRegion();
	}

	async get<A = unknown>(
		tags: ActorTags,
		opts?: GetOpts,
	): Promise<ActorHandle<A>> {
		logger().debug("get actor", { tags, opts });
		const handle = await this.#createHandle(tags, opts);
		return this.#createProxy(handle) as ActorHandle<A>;
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

	async #createHandle(
		tags: ActorTags,
		opts?: GetOpts,
	): Promise<ActorHandleRaw> {
		const create = opts?.create ?? {
			tags,
			buildTags: {
				name: tags.name,
				current: "true",
			},
			region: (await this.#regionPromise)?.id,
		};

		const resJson = await this.#sendManagerRequest<
			ActorsRequest,
			ActorsResponse
		>("POST", "/actors", {
			query: {
				getOrCreate: {
					tags,
					create,
				},
			},
		});

		const handle = new ActorHandleRaw(resJson.endpoint, opts?.parameters);
		handle.connect();
		return handle;
	}

	/** Sends an HTTP request to the manager actor. */
	async #sendManagerRequest<Request, Response>(
		method: string,
		path: string,
		body?: Request,
	): Promise<Response> {
		const managerEndpoint = await this.#managerEndpointPromise;
		const res = await fetch(`${managerEndpoint}${path}`, {
			method,
			headers: {
				"Content-Type": "application/json",
			},
			body: body ? JSON.stringify(body) : undefined,
		});

		if (!res.ok) {
			throw new Error(
				`Manager error (${res.statusText}):\n${await res.text()}`,
			);
		}

		return res.json();
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
				throw new Error(
					`Failed to fetch region (${res.statusText}):\n${await res.text()}`,
				);
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
