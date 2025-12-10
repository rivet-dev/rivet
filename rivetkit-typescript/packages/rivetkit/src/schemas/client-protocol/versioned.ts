import { createVersionedDataHandler } from "vbare";
import * as v1 from "../../../dist/schemas/client-protocol/v1";
import * as v2 from "../../../dist/schemas/client-protocol/v2";

export const CURRENT_VERSION = 2;

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

export const TO_SERVER_VERSIONED = createVersionedDataHandler<v2.ToServer>({
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
	deserializeConverters: () => [v1ToV2],
	serializeConverters: () => [v2ToV1],
});

export const TO_CLIENT_VERSIONED = createVersionedDataHandler<v2.ToClient>({
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
	deserializeConverters: () => [v1ToV2],
	serializeConverters: () => [v2ToV1],
});

export const HTTP_ACTION_REQUEST_VERSIONED =
	createVersionedDataHandler<v2.HttpActionRequest>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 1:
					return v1.decodeHttpActionRequest(bytes);
				case 2:
					return v2.decodeHttpActionRequest(bytes);
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
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		deserializeConverters: () => [],
		serializeConverters: () => [],
	});

export const HTTP_ACTION_RESPONSE_VERSIONED =
	createVersionedDataHandler<v2.HttpActionResponse>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 1:
					return v1.decodeHttpActionResponse(bytes);
				case 2:
					return v2.decodeHttpActionResponse(bytes);
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
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		deserializeConverters: () => [],
		serializeConverters: () => [],
	});

export const HTTP_RESPONSE_ERROR_VERSIONED =
	createVersionedDataHandler<v2.HttpResponseError>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 1:
					return v1.decodeHttpResponseError(bytes);
				case 2:
					return v2.decodeHttpResponseError(bytes);
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
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		deserializeConverters: () => [],
		serializeConverters: () => [],
	});

export const HTTP_RESOLVE_RESPONSE_VERSIONED =
	createVersionedDataHandler<v2.HttpResolveResponse>({
		deserializeVersion: (bytes, version) => {
			switch (version) {
				case 1:
					return v1.decodeHttpResolveResponse(bytes);
				case 2:
					return v2.decodeHttpResolveResponse(bytes);
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
				default:
					throw new Error(`Unknown version ${version}`);
			}
		},
		deserializeConverters: () => [],
		serializeConverters: () => [],
	});
