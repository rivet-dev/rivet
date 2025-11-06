import {
	createVersionedDataHandler,
	type MigrationFn,
} from "@/common/versioned-data";
import type * as v1 from "../../../dist/schemas/actor-persist/v1";
import * as v2 from "../../../dist/schemas/actor-persist/v2";

export const CURRENT_VERSION = 2;

export type CurrentPersistedActor = v2.PersistedActor;
export type CurrentPersistedConnection = v2.PersistedConnection;
export type CurrentPersistedSubscription = v2.PersistedSubscription;
export type CurrentGenericPersistedScheduleEvent =
	v2.GenericPersistedScheduleEvent;
export type CurrentPersistedScheduleEventKind = v2.PersistedScheduleEventKind;
export type CurrentPersistedScheduleEvent = v2.PersistedScheduleEvent;
export type CurrentPersistedHibernatableWebSocket =
	v2.PersistedHibernatableWebSocket;

const migrations = new Map<number, MigrationFn<any, any>>();

// Migration from v1 to v2: Add hibernatableWebSocket field
migrations.set(
	1,
	(v1Data: v1.PersistedActor): v2.PersistedActor => ({
		...v1Data,
		connections: v1Data.connections.map((conn) => ({
			...conn,
			hibernatableRequestId: null,
		})),
		hibernatableWebSocket: [],
	}),
);

export const PERSISTED_ACTOR_VERSIONED =
	createVersionedDataHandler<CurrentPersistedActor>({
		currentVersion: CURRENT_VERSION,
		migrations,
		serializeVersion: (data) => v2.encodePersistedActor(data),
		deserializeVersion: (bytes) => v2.decodePersistedActor(bytes),
	});
