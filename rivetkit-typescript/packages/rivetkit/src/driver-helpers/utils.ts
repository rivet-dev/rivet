import * as cbor from "cbor-x";
import { KEYS } from "@/actor/instance/kv";
import type * as persistSchema from "@/schemas/actor-persist/mod";
import { ACTOR_VERSIONED } from "@/schemas/actor-persist/versioned";
import { bufferToArrayBuffer } from "@/utils";
import type { ActorDriver } from "./mod";

function serializeEmptyPersistData(input: unknown | undefined): Uint8Array {
	const persistData: persistSchema.Actor = {
		input:
			input !== undefined
				? bufferToArrayBuffer(cbor.encode(input))
				: null,
		hasInitialized: false,
		state: bufferToArrayBuffer(cbor.encode(undefined)),
		hibernatableConns: [],
		scheduledEvents: [],
	};
	return ACTOR_VERSIONED.serializeWithEmbeddedVersion(persistData);
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
