import { createVersionedDataHandler } from "vbare";
import * as v1 from "../../../dist/schemas/actor-persist/v1";
import * as v2 from "../../../dist/schemas/actor-persist/v2";
import * as v3 from "../../../dist/schemas/actor-persist/v3";

export const CURRENT_VERSION = 3;

// Converter from v1 to v2
const v1ToV2 = (v1Data: v1.PersistedActor): v2.PersistedActor => ({
	...v1Data,
	connections: v1Data.connections.map((conn) => ({
		...conn,
		hibernatableRequestId: null,
	})),
	hibernatableWebSockets: [],
});

// Converter from v2 to v3
const v2ToV3 = (v2Data: v2.PersistedActor): v3.Actor => {
	// Transform scheduled events from nested structure to flat structure
	const scheduledEvents: v3.ScheduleEvent[] = v2Data.scheduledEvents.map(
		(event) => {
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
			throw new Error(`Unknown schedule event kind: ${event.kind.tag}`);
		},
	);

	return {
		input: v2Data.input,
		hasInitialized: v2Data.hasInitialized,
		state: v2Data.state,
		scheduledEvents,
	};
};

// Converter from v3 to v2
const v3ToV2 = (v3Data: v3.Actor): v2.PersistedActor => {
	// Transform scheduled events from flat structure back to nested structure
	const scheduledEvents: v2.PersistedScheduleEvent[] = v3Data.scheduledEvents.map(
		(event) => ({
			eventId: event.eventId,
			timestamp: event.timestamp,
			kind: {
				tag: "GenericPersistedScheduleEvent" as const,
				val: {
					action: event.action,
					args: event.args,
				},
			},
		}),
	);

	return {
		input: v3Data.input,
		hasInitialized: v3Data.hasInitialized,
		state: v3Data.state,
		scheduledEvents,
		connections: [],
		hibernatableWebSockets: [],
	};
};

// Converter from v2 to v1
const v2ToV1 = (v2Data: v2.PersistedActor): v1.PersistedActor => {
	return {
		input: v2Data.input,
		hasInitialized: v2Data.hasInitialized,
		state: v2Data.state,
		scheduledEvents: v2Data.scheduledEvents,
		connections: v2Data.connections.map((conn) => {
			const { hibernatableRequestId, ...rest } = conn;
			return rest;
		}),
	};
};

export const ACTOR_VERSIONED = createVersionedDataHandler<v3.Actor>({
	deserializeVersion: (bytes, version) => {
		switch (version) {
			case 1:
				return v1.decodePersistedActor(bytes);
			case 2:
				return v2.decodePersistedActor(bytes);
			case 3:
				return v3.decodeActor(bytes);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return v1.encodePersistedActor(data as v1.PersistedActor);
			case 2:
				return v2.encodePersistedActor(data as v2.PersistedActor);
			case 3:
				return v3.encodeActor(data as v3.Actor);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	deserializeConverters: () => [v1ToV2, v2ToV3],
	serializeConverters: () => [v3ToV2, v2ToV1],
});

export const CONN_VERSIONED = createVersionedDataHandler<v3.Conn>({
	deserializeVersion: (bytes, version) => {
		switch (version) {
			case 3:
				return v3.decodeConn(bytes);
			default:
				throw new Error(
					`Conn type only exists in version 3+, got version ${version}`,
				);
		}
	},
	serializeVersion: (data, version) => {
		switch (version) {
			case 3:
				return v3.encodeConn(data as v3.Conn);
			default:
				throw new Error(
					`Conn type only exists in version 3+, got version ${version}`,
				);
		}
	},
	deserializeConverters: () => [],
	serializeConverters: () => [],
});
