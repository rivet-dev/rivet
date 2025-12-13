import {
	createVersionedDataHandler,
	type MigrationFn,
} from "@/common/versioned-data";
import { bufferToArrayBuffer } from "@/utils";
import type * as v1 from "../../../dist/schemas/file-system-driver/v1";
import type * as v2 from "../../../dist/schemas/file-system-driver/v2";
import * as v3 from "../../../dist/schemas/file-system-driver/v3";

export const CURRENT_VERSION = 3;

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
	[
		3,
		(v2State: v2.ActorState): v3.ActorState => {
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
		},
	],
]);

export const ACTOR_STATE_VERSIONED = createVersionedDataHandler<v3.ActorState>({
	currentVersion: CURRENT_VERSION,
	migrations,
	serializeVersion: (data) => v3.encodeActorState(data),
	deserializeVersion: (bytes) => v3.decodeActorState(bytes),
});

export const ACTOR_ALARM_VERSIONED = createVersionedDataHandler<v3.ActorAlarm>({
	currentVersion: CURRENT_VERSION,
	migrations,
	serializeVersion: (data) => v3.encodeActorAlarm(data),
	deserializeVersion: (bytes) => v3.decodeActorAlarm(bytes),
});
