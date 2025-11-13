import {
	createVersionedDataHandler,
	type MigrationFn,
} from "@/common/versioned-data";
import type * as v1 from "../../../dist/schemas/actor-persist/v1";
import type * as v2 from "../../../dist/schemas/actor-persist/v2";
import * as v3 from "../../../dist/schemas/actor-persist/v3";

export const CURRENT_VERSION = 3;

export type CurrentPersistedActor = v3.PersistedActor;
export type CurrentPersistedHibernatableConn = v3.PersistedHibernatableConn;
export type CurrentPersistedScheduleEvent = v3.PersistedScheduleEvent;

const migrations = new Map<number, MigrationFn<any, any>>([
	[
		1,
		(v1Data: v1.PersistedActor): v2.PersistedActor => ({
			...v1Data,
			connections: v1Data.connections.map((conn) => ({
				...conn,
				hibernatableRequestId: null,
			})),
			hibernatableWebSocket: [],
		}),
	],
	[
		2,
		(v2Data: v2.PersistedActor): v3.PersistedActor => {
			// Merge connections and hibernatableWebSocket into hibernatableConns
			const hibernatableConns: v3.PersistedHibernatableConn[] = [];

			// Convert connections with hibernatable request IDs to hibernatable conns
			for (const conn of v2Data.connections) {
				if (conn.hibernatableRequestId) {
					// Find the matching hibernatable WebSocket
					const ws = v2Data.hibernatableWebSocket.find((ws) =>
						Buffer.from(ws.requestId).equals(
							Buffer.from(conn.hibernatableRequestId!),
						),
					);

					if (ws) {
						hibernatableConns.push({
							id: conn.id,
							parameters: conn.parameters,
							state: conn.state,
							hibernatableRequestId: conn.hibernatableRequestId,
							lastSeenTimestamp: ws.lastSeenTimestamp,
							msgIndex: ws.msgIndex,
						});
					}
				}
			}

			// Transform scheduled events from nested structure to flat structure
			const scheduledEvents: v3.PersistedScheduleEvent[] =
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
				hibernatableConns,
				scheduledEvents,
			};
		},
	],
]);

export const PERSISTED_ACTOR_VERSIONED =
	createVersionedDataHandler<CurrentPersistedActor>({
		currentVersion: CURRENT_VERSION,
		migrations,
		serializeVersion: (data) => v3.encodePersistedActor(data),
		deserializeVersion: (bytes) => v3.decodePersistedActor(bytes),
	});
