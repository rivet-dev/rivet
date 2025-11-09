import * as cbor from "cbor-x";
import type * as persistSchema from "@/schemas/actor-persist/mod";
import { ACTOR_VERSIONED } from "@/schemas/actor-persist/versioned";
import { bufferToArrayBuffer } from "@/utils";

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
