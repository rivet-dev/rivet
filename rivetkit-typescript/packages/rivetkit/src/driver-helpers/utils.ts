import * as cbor from "cbor-x";
import { KEYS } from "@/actor/instance/kv";
import type * as persistSchema from "@/schemas/actor-persist/mod";
import { ACTOR_VERSIONED } from "@/schemas/actor-persist/versioned";
import { bufferToArrayBuffer } from "@/utils";
import type { ActorDriver } from "./mod";

export function serializeEmptyPersistData(
	input: unknown | undefined,
): Uint8Array {
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
 * Initialize an actor's KV store with empty persist data.
 * This must be called when creating a new actor, before starting it.
 */
export async function initializeActorKv(
	driver: ActorDriver,
	actorId: string,
	input: unknown | undefined,
): Promise<void> {
	const persistData = serializeEmptyPersistData(input);
	await driver.kvBatchPut(actorId, [[KEYS.PERSIST_DATA, persistData]]);
}
