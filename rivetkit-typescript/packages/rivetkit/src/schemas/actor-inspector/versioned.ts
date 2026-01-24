import { createVersionedDataHandler } from "vbare";

import * as v1 from "../../../dist/schemas/actor-inspector/v1";
import * as v2 from "../../../dist/schemas/actor-inspector/v2";
import * as v3 from "../../../dist/schemas/actor-inspector/v3";

export const CURRENT_VERSION = 3;

// Converter from v1 to v2: Add queueSize field to Init message
const v1ToClientToV2 = (v1Data: v1.ToClient): v2.ToClient => {
	if (v1Data.body.tag === "Init") {
		const init = v1Data.body.val as v1.Init;
		return {
			body: {
				tag: "Init",
				val: {
					...init,
					queueSize: 0n,
				},
			},
		};
	}
	return v1Data as unknown as v2.ToClient;
};

// Converter from v2 to v1: Remove queueSize field from Init, filter out QueueUpdated
const v2ToClientToV1 = (v2Data: v2.ToClient): v1.ToClient => {
	if (v2Data.body.tag === "Init") {
		const init = v2Data.body.val;
		const { queueSize, ...rest } = init;
		return {
			body: {
				tag: "Init",
				val: rest,
			},
		};
	}
	// QueueUpdated doesn't exist in v1, so we can't convert it
	if (v2Data.body.tag === "QueueUpdated") {
		throw new Error("Cannot convert QueueUpdated to v1");
	}
	return v2Data as unknown as v1.ToClient;
};

// Converter from v2 to v3: Remove events from Init, drop event updates
const v2ToClientToV3 = (v2Data: v2.ToClient): v3.ToClient => {
	if (v2Data.body.tag === "Init") {
		const init = v2Data.body.val;
		const { events, ...rest } = init;
		return {
			body: {
				tag: "Init",
				val: rest,
			},
		};
	}
	if (
		v2Data.body.tag === "EventsUpdated" ||
		v2Data.body.tag === "EventsResponse"
	) {
		throw new Error("Cannot convert events responses to v3");
	}
	return v2Data as unknown as v3.ToClient;
};

// Converter from v3 to v2: Add empty events to Init, drop TraceQueryResponse
const v3ToClientToV2 = (v3Data: v3.ToClient): v2.ToClient => {
	if (v3Data.body.tag === "Init") {
		const init = v3Data.body.val;
		return {
			body: {
				tag: "Init",
				val: {
					...init,
					events: [],
				},
			},
		};
	}
	if (v3Data.body.tag === "TraceQueryResponse") {
		throw new Error("Cannot convert TraceQueryResponse to v2");
	}
	return v3Data as unknown as v2.ToClient;
};

// ToServer is identical between v1 and v2
const v1ToServerToV2 = (v1Data: v1.ToServer): v2.ToServer => {
	return v1Data as unknown as v2.ToServer;
};

const v2ToServerToV1 = (v2Data: v2.ToServer): v1.ToServer => {
	return v2Data as unknown as v1.ToServer;
};

// Converter from v2 to v3: Drop events requests
const v2ToServerToV3 = (v2Data: v2.ToServer): v3.ToServer => {
	if (
		v2Data.body.tag === "EventsRequest" ||
		v2Data.body.tag === "ClearEventsRequest"
	) {
		throw new Error("Cannot convert events requests to v3");
	}
	return v2Data as unknown as v3.ToServer;
};

// Converter from v3 to v2: Drop trace query
const v3ToServerToV2 = (v3Data: v3.ToServer): v2.ToServer => {
	if (v3Data.body.tag === "TraceQueryRequest") {
		throw new Error("Cannot convert TraceQueryRequest to v2");
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
				return v3.encodeToServer(data);
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
				return v3.encodeToClient(data);
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
