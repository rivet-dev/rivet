import { createVersionedDataHandler } from "vbare";

import * as v1 from "../../../dist/schemas/actor-inspector/v1";
import * as v2 from "../../../dist/schemas/actor-inspector/v2";
import * as v3 from "../../../dist/schemas/actor-inspector/v3";

export const CURRENT_VERSION = 3;

const EVENTS_DROPPED_ERROR = "inspector.events_dropped";
const WORKFLOW_HISTORY_DROPPED_ERROR = "inspector.workflow_history_dropped";
const QUEUE_DROPPED_ERROR = "inspector.queue_dropped";
const TRACE_DROPPED_ERROR = "inspector.trace_dropped";
const DATABASE_DROPPED_ERROR = "inspector.database_dropped";

// Converter from v1 to v2: Drop events in Init and add new fields
const v1ToClientToV2 = (v1Data: v1.ToClient): v2.ToClient => {
	if (v1Data.body.tag === "Init") {
		const init = v1Data.body.val as v1.Init;
		return {
			body: {
				tag: "Init",
				val: {
					connections: init.connections,
					state: init.state,
					isStateEnabled: init.isStateEnabled,
					rpcs: init.rpcs,
					isDatabaseEnabled: init.isDatabaseEnabled,
					queueSize: 0n,
					workflowHistory: null,
					isWorkflowEnabled: false,
				},
			},
		};
	}
	if (
		v1Data.body.tag === "EventsUpdated" ||
		v1Data.body.tag === "EventsResponse"
	) {
		return {
			body: {
				tag: "Error",
				val: {
					message: EVENTS_DROPPED_ERROR,
				},
			},
		};
	}
	return v1Data as unknown as v2.ToClient;
};

// Converter from v2 to v1: Add empty events to Init, drop newer updates
const v2ToClientToV1 = (v2Data: v2.ToClient): v1.ToClient => {
	if (v2Data.body.tag === "Init") {
		const init = v2Data.body.val;
		return {
			body: {
				tag: "Init",
				val: {
					connections: init.connections,
					events: [],
					state: init.state,
					isStateEnabled: init.isStateEnabled,
					rpcs: init.rpcs,
					isDatabaseEnabled: init.isDatabaseEnabled,
				},
			},
		};
	}
	if (
		v2Data.body.tag === "WorkflowHistoryUpdated" ||
		v2Data.body.tag === "WorkflowHistoryResponse"
	) {
		return {
			body: {
				tag: "Error",
				val: {
					message: WORKFLOW_HISTORY_DROPPED_ERROR,
				},
			},
		};
	}
	if (v2Data.body.tag === "QueueUpdated") {
		return {
			body: {
				tag: "Error",
				val: {
					message: QUEUE_DROPPED_ERROR,
				},
			},
		};
	}
	if (v2Data.body.tag === "QueueResponse") {
		return {
			body: {
				tag: "Error",
				val: {
					message: QUEUE_DROPPED_ERROR,
				},
			},
		};
	}
	if (v2Data.body.tag === "TraceQueryResponse") {
		return {
			body: {
				tag: "Error",
				val: {
					message: TRACE_DROPPED_ERROR,
				},
			},
		};
	}
	return v2Data as unknown as v1.ToClient;
};

// Converter from v2 to v3: v2 messages are a subset of v3
const v2ToClientToV3 = (v2Data: v2.ToClient): v3.ToClient => {
	return v2Data as unknown as v3.ToClient;
};

// Converter from v3 to v2: Drop database responses
const v3ToClientToV2 = (v3Data: v3.ToClient): v2.ToClient => {
	if (
		v3Data.body.tag === "DatabaseSchemaResponse" ||
		v3Data.body.tag === "DatabaseTableRowsResponse"
	) {
		return {
			body: {
				tag: "Error",
				val: {
					message: DATABASE_DROPPED_ERROR,
				},
			},
		};
	}
	return v3Data as unknown as v2.ToClient;
};

// Converter from v1 to v2: Drop events requests
const v1ToServerToV2 = (v1Data: v1.ToServer): v2.ToServer => {
	if (
		v1Data.body.tag === "EventsRequest" ||
		v1Data.body.tag === "ClearEventsRequest"
	) {
		throw new Error("Cannot convert events requests to v2");
	}
	return v1Data as unknown as v2.ToServer;
};

// Converter from v2 to v1: Drop newer requests
const v2ToServerToV1 = (v2Data: v2.ToServer): v1.ToServer => {
	if (
		v2Data.body.tag === "TraceQueryRequest" ||
		v2Data.body.tag === "QueueRequest" ||
		v2Data.body.tag === "WorkflowHistoryRequest"
	) {
		throw new Error("Cannot convert v2-only requests to v1");
	}
	return v2Data as unknown as v1.ToServer;
};

// Converter from v2 to v3: v2 messages are a subset of v3
const v2ToServerToV3 = (v2Data: v2.ToServer): v3.ToServer => {
	return v2Data as unknown as v3.ToServer;
};

// Converter from v3 to v2: Drop database requests
const v3ToServerToV2 = (v3Data: v3.ToServer): v2.ToServer => {
	if (
		v3Data.body.tag === "DatabaseSchemaRequest" ||
		v3Data.body.tag === "DatabaseTableRowsRequest"
	) {
		throw new Error("Cannot convert v3-only database requests to v2");
	}
	return v3Data as unknown as v2.ToServer;
};

export const TO_SERVER_VERSIONED = createVersionedDataHandler<v3.ToServer>({
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return v1.encodeToServer(data as v1.ToServer);
			case 2:
				return v2.encodeToServer(data as v2.ToServer);
			case 3:
				return v3.encodeToServer(data as v3.ToServer);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	deserializeVersion: (bytes, version) => {
		switch (version) {
			case 1:
				return v1.decodeToServer(bytes);
			case 2:
				return v2.decodeToServer(bytes);
			case 3:
				return v3.decodeToServer(bytes);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	deserializeConverters: () => [v1ToServerToV2, v2ToServerToV3],
	serializeConverters: () => [v3ToServerToV2, v2ToServerToV1],
});

export const TO_CLIENT_VERSIONED = createVersionedDataHandler<v3.ToClient>({
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return v1.encodeToClient(data as v1.ToClient);
			case 2:
				return v2.encodeToClient(data as v2.ToClient);
			case 3:
				return v3.encodeToClient(data as v3.ToClient);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	deserializeVersion: (bytes, version) => {
		switch (version) {
			case 1:
				return v1.decodeToClient(bytes);
			case 2:
				return v2.decodeToClient(bytes);
			case 3:
				return v3.decodeToClient(bytes);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	deserializeConverters: () => [v1ToClientToV2, v2ToClientToV3],
	serializeConverters: () => [v3ToClientToV2, v2ToClientToV1],
});
