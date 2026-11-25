import { Context } from "effect";
import type { Registry } from "../Registry.ts";

/**
 * The `Registry` service key. It lives outside `Registry.ts` so that
 * `Actor.ts` can require the registry without importing server code.
 */
export const RegistryService: Context.Service<Registry, Registry> =
	Context.Service<Registry>("@rivetkit/effect/Registry");
