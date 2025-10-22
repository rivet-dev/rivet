import {
	createVersionedDataHandler,
	type MigrationFn,
} from "@/common/versioned-data";
import * as v1 from "../../../dist/schemas/actor-persist/v1";

export const CURRENT_VERSION = 1;

export type CurrentPersistedActor = v1.PersistedActor;
export type CurrentPersistedConnection = v1.PersistedConnection;
export type CurrentPersistedSubscription = v1.PersistedSubscription;
export type CurrentGenericPersistedScheduleEvent =
	v1.GenericPersistedScheduleEvent;
export type CurrentPersistedScheduleEventKind = v1.PersistedScheduleEventKind;
export type CurrentPersistedScheduleEvent = v1.PersistedScheduleEvent;

const migrations = new Map<number, MigrationFn<any, any>>();

export const PERSISTED_ACTOR_VERSIONED =
	createVersionedDataHandler<CurrentPersistedActor>({
		currentVersion: CURRENT_VERSION,
		migrations,
		serializeVersion: (data) => v1.encodePersistedActor(data),
		deserializeVersion: (bytes) => v1.decodePersistedActor(bytes),
	});
