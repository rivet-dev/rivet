import { ActorHandleRaw } from "./handle.ts";
import { ActorTags } from "../../common/src/utils.ts";
import { ActorsRequest } from "../../manager-protocol/src/mod.ts";
import { CreateRequest } from "../../manager-protocol/src/query.ts";
import { RivetClient, RivetClientClient } from "@rivet-gg/api";

export interface WithTagsOpts {
	parameters?: unknown;
	create?: CreateRequest;
}

/**
 * Proxied wrapper of `RawActorHandle` that allows calling RPC functions
 * implicitly.
 *
 * Private methods (e.g. those starting with `_`) are automatically excluded.
 */
export type ActorHandle<A = unknown> = ActorHandleRaw & {
	[K in keyof A as K extends string ? (K extends `_${string}` ? never : K) : K]: A[K] extends (
		...args: infer Args
	) => infer Return
		? ActorRPCFunction<Args, Return>
		: never;
};

/**
 * RPC function returned by the actor proxy. This will call `ActorHandle.rpc`
 * when triggered.
 */
export type ActorRPCFunction<Args extends Array<unknown> = unknown[], Response = unknown> = (
	// Remove the first parameter, since that's `Context<...>`
	...args: Args extends [unknown, ...infer Rest] ? Rest : Args
) => Promise<Response>;

export class Client {
	private readonly client = new RivetClientClient({ token: TODO });

	private region: Promise<RivetClient.actor.Region> | null = this.#fetchRegion();

	constructor(private readonly managerEndpoint: string) {}

	async withTags<A = unknown>(tags: ActorTags, opts?: WithTagsOpts): Promise<ActorHandle<A>> {
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

	async #createHandle(tags: ActorTags, opts?: WithTagsOpts): Promise<ActorHandleRaw> {
		const create = opts?.create ?? {
			tags,
			buildTags: {
				name: tags.name,
				current: "true",
			},
			region: (await this.region).id,
		};

		//client.get("chat_room", { room: "lkjsdf" }, { noCreate: true, parameters: { token: 123 } });
		//client.get({ name: "chat_room", room: "lkjsdf" }, { noCreate: true, parameters: { token: 123 } });
		//client.get("chat_room", {
		//	tags: { channel: "foo", },
		//	parameters: { token: 123 }
		//});

		//client.withTags({ name: "chat_room", room: "lkjsdf" };
		//client.withTags("chat_room", { room: "lkjsdf" });

		// { game_mode: "tdm" } -> { game_mode: "tdm", map: "sandstorm" }

		const res = await fetch(`${this.managerEndpoint}/actors`, {
			method: "POST",
			body: JSON.stringify({
				query: {
					getOrCreate: {
						tags,
						create,
					},
				},
			} satisfies ActorsRequest),
		});

		if (!res.ok) {
			throw new Error(`Manager error (${res.statusText}):\n${await res.text()}`);
		}

		const resJson: { endpoint: string } = await res.json();
		const handle = new ActorHandleRaw(resJson.endpoint, opts?.parameters);
		handle.connect();
		return handle;
	}

	async #fetchRegion() {
		let { region } = await this.client.actor.regions.resolve({});

		return region;
	}
}
