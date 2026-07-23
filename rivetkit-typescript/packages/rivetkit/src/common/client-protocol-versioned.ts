import { createVersionedDataHandler } from "vbare";
import * as v1 from "./bare/generated/client-protocol/v1";
import * as v2 from "./bare/generated/client-protocol/v2";
import * as v3 from "./bare/generated/client-protocol/v3";
import * as v4 from "./bare/generated/client-protocol/v4";
import * as v5 from "./bare/generated/client-protocol/v5";

export const CURRENT_VERSION = 5;

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

// Converter from v2 to v3: No changes needed for ToClient.
const v2ToV3 = (v2Data: v2.ToClient): v3.ToClient => {
	return v2Data as unknown as v3.ToClient;
};

const v3ToV4 = (v3Data: v3.ToClient): v4.ToClient => {
	if (v3Data.body.tag === "Error") {
		return {
			body: {
				tag: "Error",
				val: {
					...v3Data.body.val,
					actor: null,
				},
			},
		};
	}
	return v3Data as unknown as v4.ToClient;
};

const v4ToV3 = (v4Data: v4.ToClient): v3.ToClient => {
	if (v4Data.body.tag === "Error") {
		const { actor: _, ...val } = v4Data.body.val;
		return {
			body: {
				tag: "Error",
				val,
			},
		};
	}
	return v4Data as unknown as v3.ToClient;
};

const v4ToV5 = (data: v4.ToClient): v5.ToClient =>
	data as unknown as v5.ToClient;
const v5ToV4 = (data: v5.ToClient): v4.ToClient =>
	data as unknown as v4.ToClient;

// Converter from v3 to v2: No changes needed for ToClient.
const v3ToV2 = (v3Data: v3.ToClient): v2.ToClient => {
	return v3Data as unknown as v2.ToClient;
};

// ToServer identity converters.
const v1ToServerV2 = (v1Data: v1.ToServer): v2.ToServer => {
	return v1Data as unknown as v2.ToServer;
};

const v2ToServerV3 = (v2Data: v2.ToServer): v3.ToServer => {
	return v2Data as unknown as v3.ToServer;
};

const v3ToServerV4 = (v3Data: v3.ToServer): v4.ToServer => {
	return v3Data as unknown as v4.ToServer;
};

const v4ToServerV3 = (v4Data: v4.ToServer): v3.ToServer => {
	return v4Data as unknown as v3.ToServer;
};

const v4ToServerV5 = (data: v4.ToServer): v5.ToServer =>
	data as unknown as v5.ToServer;
const v5ToServerV4 = (data: v5.ToServer): v4.ToServer =>
	data as unknown as v4.ToServer;

const v3ToServerV2 = (v3Data: v3.ToServer): v2.ToServer => {
	return v3Data as unknown as v2.ToServer;
};

const v2ToServerV1 = (v2Data: v2.ToServer): v1.ToServer => {
	return v2Data as unknown as v1.ToServer;
};

const v3HttpResponseErrorToV4 = (
	v3Data: v3.HttpResponseError,
): v4.HttpResponseError => ({
	...v3Data,
	actor: null,
});

const v4HttpResponseErrorToV3 = (
	v4Data: v4.HttpResponseError,
): v3.HttpResponseError => {
	const { actor: _, ...rest } = v4Data;
	return rest;
};

export const CLIENT_PROTOCOL_TO_SERVER =
	createVersionedDataHandler<v5.ToServer>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 1:
					return v1.decodeToServer(bytes);
				case 2:
					return v2.decodeToServer(bytes);
				case 3:
					return v3.decodeToServer(bytes);
				case 4:
					return v4.decodeToServer(bytes);
				case 5:
					return v5.decodeToServer(bytes);
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
				case 4:
					return v4.encodeToServer(data as v4.ToServer);
				case 5:
					return v5.encodeToServer(data as v5.ToServer);
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		deserializeConverters: () => [
			v1ToServerV2,
			v2ToServerV3,
			v3ToServerV4,
			v4ToServerV5,
		],
		serializeConverters: () => [
			v5ToServerV4,
			v4ToServerV3,
			v3ToServerV2,
			v2ToServerV1,
		],
	});

export const CLIENT_PROTOCOL_TO_CLIENT =
	createVersionedDataHandler<v5.ToClient>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 1:
					return v1.decodeToClient(bytes);
				case 2:
					return v2.decodeToClient(bytes);
				case 3:
					return v3.decodeToClient(bytes);
				case 4:
					return v4.decodeToClient(bytes);
				case 5:
					return v5.decodeToClient(bytes);
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
				case 4:
					return v4.encodeToClient(data as v4.ToClient);
				case 5:
					return v5.encodeToClient(data as v5.ToClient);
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		deserializeConverters: () => [v1ToV2, v2ToV3, v3ToV4, v4ToV5],
		serializeConverters: () => [v5ToV4, v4ToV3, v3ToV2, v2ToV1],
	});

export const HTTP_ACTION_REQUEST_VERSIONED =
	createVersionedDataHandler<v5.HttpActionRequest>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 1:
					return v1.decodeHttpActionRequest(bytes);
				case 2:
					return v2.decodeHttpActionRequest(bytes);
				case 3:
					return v3.decodeHttpActionRequest(bytes);
				case 4:
					return v4.decodeHttpActionRequest(bytes);
				case 5:
					return v5.decodeHttpActionRequest(bytes);
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
				case 4:
					return v4.encodeHttpActionRequest(
						data as v4.HttpActionRequest,
					);
				case 5:
					return v5.encodeHttpActionRequest(
						data as v5.HttpActionRequest,
					);
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		deserializeConverters: () => [],
		serializeConverters: () => [],
	});

export const HTTP_ACTION_RESPONSE_VERSIONED =
	createVersionedDataHandler<v5.HttpActionResponse>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 1:
					return v1.decodeHttpActionResponse(bytes);
				case 2:
					return v2.decodeHttpActionResponse(bytes);
				case 3:
					return v3.decodeHttpActionResponse(bytes);
				case 4:
					return v4.decodeHttpActionResponse(bytes);
				case 5:
					return v5.decodeHttpActionResponse(bytes);
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
				case 4:
					return v4.encodeHttpActionResponse(
						data as v4.HttpActionResponse,
					);
				case 5:
					return v5.encodeHttpActionResponse(
						data as v5.HttpActionResponse,
					);
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		deserializeConverters: () => [],
		serializeConverters: () => [],
	});

export const HTTP_QUEUE_SEND_REQUEST_VERSIONED =
	createVersionedDataHandler<v5.HttpQueueSendRequest>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 3:
					return v3.decodeHttpQueueSendRequest(bytes);
				case 4:
					return v4.decodeHttpQueueSendRequest(bytes);
				case 5:
					return v5.decodeHttpQueueSendRequest(bytes);
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
				case 4:
					return v4.encodeHttpQueueSendRequest(
						data as v4.HttpQueueSendRequest,
					);
				case 5:
					return v5.encodeHttpQueueSendRequest(
						data as v5.HttpQueueSendRequest,
					);
				default:
					throw new Error(
						`HttpQueueSendRequest only exists in version 3+, got version ${version}`,
					);
			}
		},
		deserializeConverters: () => [
			(data: v3.HttpQueueSendRequest) =>
				data as unknown as v4.HttpQueueSendRequest,
			(data: v4.HttpQueueSendRequest): v5.HttpQueueSendRequest => {
				if (data.wait) {
					throw new Error("queue wait is not supported by protocol v5");
				}
				return {
					body: data.body,
					name: data.name,
					dedupeKey: null,
					delay: null,
				};
			},
		],
		serializeConverters: () => [
			(data: v5.HttpQueueSendRequest): v4.HttpQueueSendRequest => ({
				body: data.body,
				name: data.name,
				wait: false,
				timeout: null,
			}),
			(data: v4.HttpQueueSendRequest) =>
				data as unknown as v3.HttpQueueSendRequest,
		],
	});

export const HTTP_QUEUE_SEND_RESPONSE_VERSIONED =
	createVersionedDataHandler<v5.HttpQueueSendResponse>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 3:
					return v3.decodeHttpQueueSendResponse(bytes);
				case 4:
					return v4.decodeHttpQueueSendResponse(bytes);
				case 5:
					return v5.decodeHttpQueueSendResponse(bytes);
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
				case 4:
					return v4.encodeHttpQueueSendResponse(
						data as v4.HttpQueueSendResponse,
					);
				case 5:
					return v5.encodeHttpQueueSendResponse(
						data as v5.HttpQueueSendResponse,
					);
				default:
					throw new Error(
						`HttpQueueSendResponse only exists in version 3+, got version ${version}`,
					);
			}
		},
		deserializeConverters: () => [
			(data: v3.HttpQueueSendResponse) =>
				data as unknown as v4.HttpQueueSendResponse,
			(_data: v4.HttpQueueSendResponse): v5.HttpQueueSendResponse => ({
				receiptId: "",
				deduplicated: false,
			}),
		],
		serializeConverters: () => [
			(_data: v5.HttpQueueSendResponse): v4.HttpQueueSendResponse => ({
				status: "completed",
				response: null,
			}),
			(data: v4.HttpQueueSendResponse) =>
				data as unknown as v3.HttpQueueSendResponse,
		],
	});

export const HTTP_QUEUE_STATUS_RESPONSE_VERSIONED =
	createVersionedDataHandler<v5.HttpQueueStatusResponse>({
		deserializeVersion: (bytes, version) => {
			if (version !== 5) {
				throw new Error(
					`HttpQueueStatusResponse only exists in version 5+, got version ${version}`,
				);
			}
			return v5.decodeHttpQueueStatusResponse(bytes);
		},
		serializeVersion: (data, version) => {
			if (version !== 5) {
				throw new Error(
					`HttpQueueStatusResponse only exists in version 5+, got version ${version}`,
				);
			}
			return v5.encodeHttpQueueStatusResponse(data);
		},
		deserializeConverters: () => [],
		serializeConverters: () => [],
	});

export const HTTP_RESPONSE_ERROR_VERSIONED =
	createVersionedDataHandler<v5.HttpResponseError>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 1:
					return v1.decodeHttpResponseError(bytes);
				case 2:
					return v2.decodeHttpResponseError(bytes);
				case 3:
					return v3.decodeHttpResponseError(bytes);
				case 4:
					return v4.decodeHttpResponseError(bytes);
				case 5:
					return v5.decodeHttpResponseError(bytes);
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
				case 4:
					return v4.encodeHttpResponseError(
						data as v4.HttpResponseError,
					);
				case 5:
					return v5.encodeHttpResponseError(
						data as v5.HttpResponseError,
					);
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		deserializeConverters: () => [
			(data: v1.HttpResponseError) =>
				data as unknown as v2.HttpResponseError,
			(data: v2.HttpResponseError) =>
				data as unknown as v3.HttpResponseError,
			v3HttpResponseErrorToV4,
			(data: v4.HttpResponseError) =>
				data as unknown as v5.HttpResponseError,
		],
		serializeConverters: () => [
			(data: v5.HttpResponseError) =>
				data as unknown as v4.HttpResponseError,
			v4HttpResponseErrorToV3,
			(data: v3.HttpResponseError) =>
				data as unknown as v2.HttpResponseError,
			(data: v2.HttpResponseError) =>
				data as unknown as v1.HttpResponseError,
		],
	});

export const HTTP_RESOLVE_RESPONSE_VERSIONED =
	createVersionedDataHandler<v5.HttpResolveResponse>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 1:
					return v1.decodeHttpResolveResponse(bytes);
				case 2:
					return v2.decodeHttpResolveResponse(bytes);
				case 3:
					return v3.decodeHttpResolveResponse(bytes);
				case 4:
					return v4.decodeHttpResolveResponse(bytes);
				case 5:
					return v5.decodeHttpResolveResponse(bytes);
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
				case 4:
					return v4.encodeHttpResolveResponse(
						data as v4.HttpResolveResponse,
					);
				case 5:
					return v5.encodeHttpResolveResponse(
						data as v5.HttpResolveResponse,
					);
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		deserializeConverters: () => [],
		serializeConverters: () => [],
	});
