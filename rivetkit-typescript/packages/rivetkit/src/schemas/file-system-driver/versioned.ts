import {
	createVersionedDataHandler,
	type MigrationFn,
} from "@/common/versioned-data";
import * as v1 from "../../../dist/schemas/file-system-driver/v1";

export const CURRENT_VERSION = 1;

export type CurrentActorState = v1.ActorState;
export type CurrentActorAlarm = v1.ActorAlarm;

const migrations = new Map<number, MigrationFn<any, any>>();

export const ACTOR_STATE_VERSIONED =
	createVersionedDataHandler<CurrentActorState>({
		currentVersion: CURRENT_VERSION,
		migrations,
		serializeVersion: (data) => v1.encodeActorState(data),
		deserializeVersion: (bytes) => v1.decodeActorState(bytes),
	});

export const ACTOR_ALARM_VERSIONED =
	createVersionedDataHandler<CurrentActorAlarm>({
		currentVersion: CURRENT_VERSION,
		migrations,
		serializeVersion: (data) => v1.encodeActorAlarm(data),
		deserializeVersion: (bytes) => v1.decodeActorAlarm(bytes),
	});
