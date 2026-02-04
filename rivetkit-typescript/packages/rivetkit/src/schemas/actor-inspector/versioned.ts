import { createVersionedDataHandler } from "vbare";

import * as v1 from "../../../dist/schemas/actor-inspector/v1";
import * as v2 from "../../../dist/schemas/actor-inspector/v2";

export const CURRENT_VERSION = 2;

const EVENTS_DROPPED_ERROR = "inspector.events_dropped";
const WORKFLOW_HISTORY_DROPPED_ERROR = "inspector.workflow_history_dropped";
const QUEUE_DROPPED_ERROR = "inspector.queue_dropped";
const TRACE_DROPPED_ERROR = "inspector.trace_dropped";

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

export const TO_SERVER_VERSIONED = createVersionedDataHandler<v2.ToServer>({
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return v1.encodeToServer(data as v1.ToServer);
			case 2:
				return v2.encodeToServer(data as v2.ToServer);
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
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	deserializeConverters: () => [v1ToServerToV2],
	serializeConverters: () => [v2ToServerToV1],
});

export const TO_CLIENT_VERSIONED = createVersionedDataHandler<v2.ToClient>({
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return v1.encodeToClient(data as v1.ToClient);
			case 2:
				return v2.encodeToClient(data as v2.ToClient);
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
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	deserializeConverters: () => [v1ToClientToV2],
	serializeConverters: () => [v2ToClientToV1],
});
