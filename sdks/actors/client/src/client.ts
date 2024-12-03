import { ActorHandleRaw } from "./handle.ts";
import { ActorTags } from "../../common/src/utils.ts";
import { ActorsRequest } from "../../manager-protocol/src/mod.ts";
import { CreateRequest } from "../../manager-protocol/src/query.ts";

export interface WithTagsOpts {
	parameters?: unknown;
	create?: CreateRequest;
}

/** RPC function returned by the actor proxy. This will call `ActorHandle.rpc`
 * when triggered. */
export type ActorRPCFunction<
	Args extends Array<unknown> = unknown[],
	Response = unknown,
> = (...args: Args) => Promise<Response>;

/** Proxied wrapper of `RawActorHandle` that allows calling RPC functions
 * implicitly. */
export type ActorHandle<A = unknown> =
	& ActorHandleRaw
	& {
		[K in keyof A]: A[K] extends (...args: infer Args) => infer Return
			? (...args: Args) => Promise<Return>
			: never;
	};

export class ActorClient {
	constructor(private readonly managerEndpoint: string) {}

	async withTags<A = unknown>(
		tags: ActorTags,
		opts?: WithTagsOpts,
	): Promise<ActorHandle<A>> {
		const handle = await this.#createHandle(tags, opts);
		return this.#createProxy(handle) as ActorHandle<A>;
	}

	#createProxy(handle: ActorHandleRaw): ActorHandle {
		// Stores returned RPC functions for faster calls
		const methodCache = new Map<string, ActorRPCFunction>();

		return new Proxy(handle, {
			get(target: ActorHandleRaw, prop: string | symbol, receiver: any) {
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
		opts?: WithTagsOpts,
	): Promise<ActorHandleRaw> {
		const create = opts?.create ?? {
			tags,
			buildTags: {
				...tags,
				current: "true",
			},
		};

		const res = await fetch(`${this.managerEndpoint}/actors`, {
			method: "POST",
			body: JSON.stringify(
				{
					query: {
						getOrCreate: {
							tags,
							create,
						},
					},
				} satisfies ActorsRequest,
			),
		});

		if (!res.ok) {
			throw new Error(
				`Manager error (${res.statusText}):\n${await res.text()}`,
			);
		}

		const resJson: { endpoint: string } = await res.json();
		const handle = new ActorHandleRaw(resJson.endpoint, opts?.parameters);
		handle.connect();
		return handle;
	}
}
