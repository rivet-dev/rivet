import { createVersionedDataHandler } from "vbare";
import * as v1 from "../../../dist/schemas/client-protocol/v1";
import * as v2 from "../../../dist/schemas/client-protocol/v2";
import * as v3 from "../../../dist/schemas/client-protocol/v3";

export const CURRENT_VERSION = 3;

// Converter from v1 to v2: Remove connectionToken from Init message
const v1ToV2 = (v1Data: v1.ToClient): v2.ToClient => {
	// Handle Init message specifically to remove connectionToken
	if (v1Data.body.tag === "Init") {
		const { actorId, connectionId } = v1Data.body.val as v1.Init;
		return {
			body: {
				tag: "Init",
				val: {
					actorId,
					connectionId,
				},
			},
		};
	}
	// All other messages are unchanged
	return v1Data as unknown as v2.ToClient;
};

// Converter from v2 to v1: Add empty connectionToken to Init message
const v2ToV1 = (v2Data: v2.ToClient): v1.ToClient => {
	// Handle Init message specifically to add connectionToken
	if (v2Data.body.tag === "Init") {
		const { actorId, connectionId } = v2Data.body.val;
		return {
			body: {
				tag: "Init",
				val: {
					actorId,
					connectionId,
					connectionToken: "", // Add empty connectionToken for v1 compatibility
				},
			},
		};
	}
	// All other messages are unchanged
	return v2Data as unknown as v1.ToClient;
};

// Converter from v2 to v3: No changes needed for ToClient
const v2ToV3 = (v2Data: v2.ToClient): v3.ToClient => {
	return v2Data as unknown as v3.ToClient;
};

// Converter from v3 to v2: No changes needed for ToClient
const v3ToV2 = (v3Data: v3.ToClient): v2.ToClient => {
	return v3Data as unknown as v2.ToClient;
};

// ToServer identity converters (ToServer is identical across v1, v2, and v3)
const v1ToServerV2 = (v1Data: v1.ToServer): v2.ToServer => {
	return v1Data as unknown as v2.ToServer;
};

const v2ToServerV3 = (v2Data: v2.ToServer): v3.ToServer => {
	return v2Data as unknown as v3.ToServer;
};

const v3ToServerV2 = (v3Data: v3.ToServer): v2.ToServer => {
	return v3Data as unknown as v2.ToServer;
};

const v2ToServerV1 = (v2Data: v2.ToServer): v1.ToServer => {
	return v2Data as unknown as v1.ToServer;
};

export const TO_SERVER_VERSIONED = createVersionedDataHandler<v3.ToServer>({
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
	deserializeConverters: () => [v1ToServerV2, v2ToServerV3],
	serializeConverters: () => [v3ToServerV2, v2ToServerV1],
});

export const TO_CLIENT_VERSIONED = createVersionedDataHandler<v3.ToClient>({
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
	deserializeConverters: () => [v1ToV2, v2ToV3],
	serializeConverters: () => [v3ToV2, v2ToV1],
});

export const HTTP_ACTION_REQUEST_VERSIONED =
	createVersionedDataHandler<v3.HttpActionRequest>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 1:
					return v1.decodeHttpActionRequest(bytes);
				case 2:
					return v2.decodeHttpActionRequest(bytes);
				case 3:
					return v3.decodeHttpActionRequest(bytes);
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		serializeVersion: (data, version) => {
			switch (version) {
				case 1:
					return v1.encodeHttpActionRequest(
						data as v1.HttpActionRequest,
					);
				case 2:
					return v2.encodeHttpActionRequest(
						data as v2.HttpActionRequest,
					);
				case 3:
					return v3.encodeHttpActionRequest(
						data as v3.HttpActionRequest,
					);
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		deserializeConverters: () => [],
		serializeConverters: () => [],
	});

export const HTTP_ACTION_RESPONSE_VERSIONED =
	createVersionedDataHandler<v3.HttpActionResponse>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 1:
					return v1.decodeHttpActionResponse(bytes);
				case 2:
					return v2.decodeHttpActionResponse(bytes);
				case 3:
					return v3.decodeHttpActionResponse(bytes);
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		serializeVersion: (data, version) => {
			switch (version) {
				case 1:
					return v1.encodeHttpActionResponse(
						data as v1.HttpActionResponse,
					);
				case 2:
					return v2.encodeHttpActionResponse(
						data as v2.HttpActionResponse,
					);
				case 3:
					return v3.encodeHttpActionResponse(
						data as v3.HttpActionResponse,
					);
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		deserializeConverters: () => [],
		serializeConverters: () => [],
	});

export const HTTP_QUEUE_SEND_REQUEST_VERSIONED =
	createVersionedDataHandler<v3.HttpQueueSendRequest>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 3:
					return v3.decodeHttpQueueSendRequest(bytes);
				default:
					throw new Error(
						`HttpQueueSendRequest only exists in version 3+, got version ${version}`,
					);
			}
		},
		serializeVersion: (data, version) => {
			switch (version) {
				case 3:
					return v3.encodeHttpQueueSendRequest(
						data as v3.HttpQueueSendRequest,
					);
				default:
					throw new Error(
						`HttpQueueSendRequest only exists in version 3+, got version ${version}`,
					);
			}
		},
		deserializeConverters: () => [],
		serializeConverters: () => [],
	});

export const HTTP_QUEUE_SEND_RESPONSE_VERSIONED =
	createVersionedDataHandler<v3.HttpQueueSendResponse>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 3:
					return v3.decodeHttpQueueSendResponse(bytes);
				default:
					throw new Error(
						`HttpQueueSendResponse only exists in version 3+, got version ${version}`,
					);
			}
		},
		serializeVersion: (data, version) => {
			switch (version) {
				case 3:
					return v3.encodeHttpQueueSendResponse(
						data as v3.HttpQueueSendResponse,
					);
				default:
					throw new Error(
						`HttpQueueSendResponse only exists in version 3+, got version ${version}`,
					);
			}
		},
		deserializeConverters: () => [],
		serializeConverters: () => [],
	});

export const HTTP_RESPONSE_ERROR_VERSIONED =
	createVersionedDataHandler<v3.HttpResponseError>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 1:
					return v1.decodeHttpResponseError(bytes);
				case 2:
					return v2.decodeHttpResponseError(bytes);
				case 3:
					return v3.decodeHttpResponseError(bytes);
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		serializeVersion: (data, version) => {
			switch (version) {
				case 1:
					return v1.encodeHttpResponseError(
						data as v1.HttpResponseError,
					);
				case 2:
					return v2.encodeHttpResponseError(
						data as v2.HttpResponseError,
					);
				case 3:
					return v3.encodeHttpResponseError(
						data as v3.HttpResponseError,
					);
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		deserializeConverters: () => [],
		serializeConverters: () => [],
	});

export const HTTP_RESOLVE_RESPONSE_VERSIONED =
	createVersionedDataHandler<v3.HttpResolveResponse>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 1:
					return v1.decodeHttpResolveResponse(bytes);
				case 2:
					return v2.decodeHttpResolveResponse(bytes);
				case 3:
					return v3.decodeHttpResolveResponse(bytes);
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		serializeVersion: (data, version) => {
			switch (version) {
				case 1:
					return v1.encodeHttpResolveResponse(
						data as v1.HttpResolveResponse,
					);
				case 2:
					return v2.encodeHttpResolveResponse(
						data as v2.HttpResolveResponse,
					);
				case 3:
					return v3.encodeHttpResolveResponse(
						data as v3.HttpResolveResponse,
					);
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		deserializeConverters: () => [],
		serializeConverters: () => [],
	});
