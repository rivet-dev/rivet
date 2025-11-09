import {
	createVersionedDataHandler,
	type MigrationFn,
} from "@/common/versioned-data";
import type * as v1 from "../../../dist/schemas/client-protocol/v1";
import * as v2 from "../../../dist/schemas/client-protocol/v2";

export const CURRENT_VERSION = 2;

const migrations = new Map<number, MigrationFn<any, any>>();

// Migration from v1 to v2: Remove connectionToken from Init message
migrations.set(1, (v1Data: v1.ToClient): v2.ToClient => {
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
});

export const TO_SERVER_VERSIONED = createVersionedDataHandler<v2.ToServer>({
	currentVersion: CURRENT_VERSION,
	migrations,
	serializeVersion: (data) => v2.encodeToServer(data),
	deserializeVersion: (bytes) => v2.decodeToServer(bytes),
});

export const TO_CLIENT_VERSIONED = createVersionedDataHandler<v2.ToClient>({
	currentVersion: CURRENT_VERSION,
	migrations,
	serializeVersion: (data) => v2.encodeToClient(data),
	deserializeVersion: (bytes) => v2.decodeToClient(bytes),
});

export const HTTP_ACTION_REQUEST_VERSIONED =
	createVersionedDataHandler<v2.HttpActionRequest>({
		currentVersion: CURRENT_VERSION,
		migrations,
		serializeVersion: (data) => v2.encodeHttpActionRequest(data),
		deserializeVersion: (bytes) => v2.decodeHttpActionRequest(bytes),
	});

export const HTTP_ACTION_RESPONSE_VERSIONED =
	createVersionedDataHandler<v2.HttpActionResponse>({
		currentVersion: CURRENT_VERSION,
		migrations,
		serializeVersion: (data) => v2.encodeHttpActionResponse(data),
		deserializeVersion: (bytes) => v2.decodeHttpActionResponse(bytes),
	});

export const HTTP_RESPONSE_ERROR_VERSIONED =
	createVersionedDataHandler<v2.HttpResponseError>({
		currentVersion: CURRENT_VERSION,
		migrations,
		serializeVersion: (data) => v2.encodeHttpResponseError(data),
		deserializeVersion: (bytes) => v2.decodeHttpResponseError(bytes),
	});

export const HTTP_RESOLVE_REQUEST_VERSIONED =
	createVersionedDataHandler<v2.HttpResolveRequest>({
		currentVersion: CURRENT_VERSION,
		migrations,
		serializeVersion: (_) => new Uint8Array(),
		deserializeVersion: (bytes) => null,
	});

export const HTTP_RESOLVE_RESPONSE_VERSIONED =
	createVersionedDataHandler<v2.HttpResolveResponse>({
		currentVersion: CURRENT_VERSION,
		migrations,
		serializeVersion: (data) => v2.encodeHttpResolveResponse(data),
		deserializeVersion: (bytes) => v2.decodeHttpResolveResponse(bytes),
	});
