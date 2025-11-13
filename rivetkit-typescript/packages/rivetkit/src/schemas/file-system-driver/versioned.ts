import {
	createVersionedDataHandler,
	type MigrationFn,
} from "@/common/versioned-data";
import { bufferToArrayBuffer } from "@/utils";
import type * as v1 from "../../../dist/schemas/file-system-driver/v1";
import * as v2 from "../../../dist/schemas/file-system-driver/v2";

export const CURRENT_VERSION = 2;

const migrations = new Map<number, MigrationFn<any, any>>([
	[
		2,
		(v1State: v1.ActorState): v2.ActorState => {
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
		},
	],
]);

export const ACTOR_STATE_VERSIONED = createVersionedDataHandler<v2.ActorState>({
	currentVersion: CURRENT_VERSION,
	migrations,
	serializeVersion: (data) => v2.encodeActorState(data),
	deserializeVersion: (bytes) => v2.decodeActorState(bytes),
});

export const ACTOR_ALARM_VERSIONED = createVersionedDataHandler<v2.ActorAlarm>({
	currentVersion: CURRENT_VERSION,
	migrations,
	serializeVersion: (data) => v2.encodeActorAlarm(data),
	deserializeVersion: (bytes) => v2.decodeActorAlarm(bytes),
});
