import * as cbor from "cbor-x";
import { KEYS } from "@/actor/instance/keys";
import type * as persistSchema from "@/schemas/actor-persist/mod";
import {
	ACTOR_VERSIONED,
	CURRENT_VERSION,
} from "@/schemas/actor-persist/versioned";
import { bufferToArrayBuffer } from "@/utils";
import type { ActorDriver } from "./mod";
import type { SqliteVfs } from "@rivetkit/sqlite-vfs";

function serializeEmptyPersistData(input: unknown | undefined): Uint8Array {
	const persistData: persistSchema.Actor = {
		input:
			input !== undefined
				? bufferToArrayBuffer(cbor.encode(input))
				: null,
		hasInitialized: false,
		state: bufferToArrayBuffer(cbor.encode(undefined)),
		scheduledEvents: [],
	};
	return ACTOR_VERSIONED.serializeWithEmbeddedVersion(
		persistData,
		CURRENT_VERSION,
	);
}

/**
 * Returns the initial KV state for a new actor. This is ued by the drivers to
 * write the initial state in to KV storage before starting the actor.
 */
export function getInitialActorKvState(
	input: unknown | undefined,
): [Uint8Array, Uint8Array][] {
	const persistData = serializeEmptyPersistData(input);
	return [[KEYS.PERSIST_DATA, persistData]];
}

/**
 * Dynamically import @rivetkit/sqlite-vfs and return a fresh SqliteVfs instance.
 *
 * The module specifier is built with Array.join() so that bundlers (esbuild, tsup,
 * Turbopack) cannot statically analyze or constant-fold the import path. This
 * prevents them from tracing into the WASM dependency tree, which would cause
 * errors in environments that don't support .wasm imports (e.g. Turbopack).
 *
 * Each call returns a new instance so that actors get independent SQLite modules,
 * avoiding cross-actor re-entry on the non-reentrant async build.
 */
export async function importSqliteVfs(): Promise<SqliteVfs> {
	const specifier = ["@rivetkit", "sqlite-vfs"].join("/");
	const { SqliteVfs } = await import(specifier);
	return new SqliteVfs();
}
