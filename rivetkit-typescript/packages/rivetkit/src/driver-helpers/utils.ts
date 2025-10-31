import * as cbor from "cbor-x";
import type * as schema from "@/schemas/actor-persist/mod";
import { PERSISTED_ACTOR_VERSIONED } from "@/schemas/actor-persist/versioned";
import { bufferToArrayBuffer } from "@/utils";

export function serializeEmptyPersistData(
	input: unknown | undefined,
): Uint8Array {
	const persistData: schema.PersistedActor = {
		input:
			input !== undefined
				? bufferToArrayBuffer(cbor.encode(input))
				: null,
		hasInitialized: false,
		state: bufferToArrayBuffer(cbor.encode(undefined)),
		connections: [],
		scheduledEvents: [],
		hibernatableWebSocket: [],
	};
	return PERSISTED_ACTOR_VERSIONED.serializeWithEmbeddedVersion(persistData);
}
