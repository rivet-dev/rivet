import { createVersionedDataHandler } from "vbare";
import { bufferToArrayBuffer } from "@/utils";
import * as v1 from "../../../dist/schemas/file-system-driver/v1";
import * as v2 from "../../../dist/schemas/file-system-driver/v2";
import * as v3 from "../../../dist/schemas/file-system-driver/v3";

export const CURRENT_VERSION = 3;

// Converter from v1 to v2
const v1ToV2 = (v1State: v1.ActorState): v2.ActorState => {
	// Create a new kvStorage list with the legacy persist data
	const kvStorage: v2.ActorKvEntry[] = [];

	// Store the legacy persist data under key [1]
	if (v1State.persistedData) {
		// Key [1] as Uint8Array
		const key = new Uint8Array([1]);
		kvStorage.push({
			key: bufferToArrayBuffer(key),
			value: v1State.persistedData,
		});
	}

	return {
		actorId: v1State.actorId,
		name: v1State.name,
		key: v1State.key,
		kvStorage,
		createdAt: v1State.createdAt,
	};
};

// Converter from v2 to v3
const v2ToV3 = (v2State: v2.ActorState): v3.ActorState => {
	// Migrate from v2 to v3 by adding the new optional timestamp fields
	return {
		actorId: v2State.actorId,
		name: v2State.name,
		key: v2State.key,
		kvStorage: v2State.kvStorage,
		createdAt: v2State.createdAt,
		startTs: null,
		connectableTs: null,
		sleepTs: null,
		destroyTs: null,
	};
};

// Converter from v3 to v2
const v3ToV2 = (v3State: v3.ActorState): v2.ActorState => {
	// Downgrade from v3 to v2 by removing the timestamp fields
	return {
		actorId: v3State.actorId,
		name: v3State.name,
		key: v3State.key,
		kvStorage: v3State.kvStorage,
		createdAt: v3State.createdAt,
	};
};

// Converter from v2 to v1
const v2ToV1 = (v2State: v2.ActorState): v1.ActorState => {
	// Downgrade from v2 to v1 by converting kvStorage back to persistedData
	// Find the persist data entry (key [1])
	const persistDataEntry = v2State.kvStorage.find((entry) => {
		const key = new Uint8Array(entry.key);
		return key.length === 1 && key[0] === 1;
	});

	return {
		actorId: v2State.actorId,
		name: v2State.name,
		key: v2State.key,
		persistedData: persistDataEntry?.value || new ArrayBuffer(0),
		createdAt: v2State.createdAt,
	};
};

export const ACTOR_STATE_VERSIONED = createVersionedDataHandler<v3.ActorState>({
	deserializeVersion: (bytes, version) => {
		switch (version) {
			case 1:
				return v1.decodeActorState(bytes);
			case 2:
				return v2.decodeActorState(bytes);
			case 3:
				return v3.decodeActorState(bytes);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return v1.encodeActorState(data as v1.ActorState);
			case 2:
				return v2.encodeActorState(data as v2.ActorState);
			case 3:
				return v3.encodeActorState(data as v3.ActorState);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	deserializeConverters: () => [v1ToV2, v2ToV3],
	serializeConverters: () => [v3ToV2, v2ToV1],
});

export const ACTOR_ALARM_VERSIONED = createVersionedDataHandler<v3.ActorAlarm>({
	deserializeVersion: (bytes, version) => {
		switch (version) {
			case 1:
				return v1.decodeActorAlarm(bytes);
			case 2:
				return v2.decodeActorAlarm(bytes);
			case 3:
				return v3.decodeActorAlarm(bytes);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return v1.encodeActorAlarm(data as v1.ActorAlarm);
			case 2:
				return v2.encodeActorAlarm(data as v2.ActorAlarm);
			case 3:
				return v3.encodeActorAlarm(data as v3.ActorAlarm);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	deserializeConverters: () => [],
	serializeConverters: () => [],
});
