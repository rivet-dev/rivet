import {
	createVersionedDataHandler,
	type MigrationFn,
} from "@/common/versioned-data";
import type * as v1 from "../../../dist/schemas/actor-persist/v1";
import type * as v2 from "../../../dist/schemas/actor-persist/v2";
import * as v3 from "../../../dist/schemas/actor-persist/v3";

export const CURRENT_VERSION = 3;

const migrations = new Map<number, MigrationFn<any, any>>([
	[
		1,
		(v1Data: v1.PersistedActor): v2.PersistedActor => ({
			...v1Data,
			connections: v1Data.connections.map((conn) => ({
				...conn,
				hibernatableRequestId: null,
			})),
			hibernatableWebSockets: [],
		}),
	],
	[
		2,
		(v2Data: v2.PersistedActor): v3.Actor => {
			// Transform scheduled events from nested structure to flat structure
			const scheduledEvents: v3.ScheduleEvent[] =
				v2Data.scheduledEvents.map((event) => {
					// Extract action and args from the kind wrapper
					if (event.kind.tag === "GenericPersistedScheduleEvent") {
						return {
							eventId: event.eventId,
							timestamp: event.timestamp,
							action: event.kind.val.action,
							args: event.kind.val.args,
						};
					}
					// Fallback for unknown kinds
					throw new Error(
						`Unknown schedule event kind: ${event.kind.tag}`,
					);
				});

			return {
				input: v2Data.input,
				hasInitialized: v2Data.hasInitialized,
				state: v2Data.state,
				scheduledEvents,
			};
		},
	],
]);

export const ACTOR_VERSIONED = createVersionedDataHandler<v3.Actor>({
	currentVersion: CURRENT_VERSION,
	migrations,
	serializeVersion: (data) => v3.encodeActor(data),
	deserializeVersion: (bytes) => v3.decodeActor(bytes),
});

export const CONN_VERSIONED = createVersionedDataHandler<v3.Conn>({
	currentVersion: CURRENT_VERSION,
	migrations: new Map(),
	serializeVersion: (data) => v3.encodeConn(data),
	deserializeVersion: (bytes) => v3.decodeConn(bytes),
});
