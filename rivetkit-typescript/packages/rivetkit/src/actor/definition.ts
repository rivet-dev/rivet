import type { RegistryConfig } from "@/registry/config";
import { getRequireFn } from "@/utils/node";
import type { Actions, ActorConfig } from "./config";
import type { AnyDatabaseProvider } from "./database";
import type { ActorInstance } from "./instance/mod";

export type AnyActorDefinition = ActorDefinition<
	any,
	any,
	any,
	any,
	any,
	any,
	any
>;

export class ActorDefinition<
	S,
	CP,
	CS,
	V,
	I,
	DB extends AnyDatabaseProvider,
	R extends Actions<S, CP, CS, V, I, DB>,
> {
	#config: ActorConfig<S, CP, CS, V, I, DB>;

	constructor(config: ActorConfig<S, CP, CS, V, I, DB>) {
		this.#config = config;
	}

	get config(): ActorConfig<S, CP, CS, V, I, DB> {
		return this.#config;
	}

	async instantiate(): Promise<ActorInstance<S, CP, CS, V, I, DB>> {
		// Lazy import to avoid pulling server-only dependencies (traces, fdb-tuple, etc.)
		// into browser bundles. This method is only called on the server.
		// const requireFn = getRequireFn();
		// if (!requireFn) {
		// 	throw new Error(
		// 		"ActorDefinition.instantiate requires a Node.js environment",
		// 	);
		// }

		try {
			const { ActorInstance: ActorInstanceClass } = await import(
				"./instance/mod"
			);
			return new ActorInstanceClass(this.#config);
		} catch (error) {
			if (!isInstanceModuleNotFound(error)) {
				throw error;
			}

			try {
				// In tests, register tsx so require() can resolve .ts files.
				await getRequireFn()("tsx/cjs");
			} catch {
				throw error;
			}

			const { ActorInstance: ActorInstanceClass } = await import(
				"./instance/mod"
			);
			return new ActorInstanceClass(this.#config);
		}
	}
}

function isInstanceModuleNotFound(error: unknown): boolean {
	if (!error || typeof error !== "object") return false;
	const err = error as { code?: string; message?: string };
	if (err.code !== "MODULE_NOT_FOUND") return false;
	return (err.message ?? "").includes("./instance/mod");
}

export function lookupInRegistry(
	config: RegistryConfig,
	name: string,
): AnyActorDefinition {
	// Build actor
	const definition = config.use[name];
	if (!definition) throw new Error(`no actor in registry for name ${name}`);
	return definition;
}
