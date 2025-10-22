import {
	createVersionedDataHandler,
	type MigrationFn,
} from "@/common/versioned-data";
import * as v1 from "../../../dist/schemas/client-protocol/v1";

export const CURRENT_VERSION = 1;

const migrations = new Map<number, MigrationFn<any, any>>();

export const TO_SERVER_VERSIONED = createVersionedDataHandler<v1.ToServer>({
	currentVersion: CURRENT_VERSION,
	migrations,
	serializeVersion: (data) => v1.encodeToServer(data),
	deserializeVersion: (bytes) => v1.decodeToServer(bytes),
});

export const TO_CLIENT_VERSIONED = createVersionedDataHandler<v1.ToClient>({
	currentVersion: CURRENT_VERSION,
	migrations,
	serializeVersion: (data) => v1.encodeToClient(data),
	deserializeVersion: (bytes) => v1.decodeToClient(bytes),
});

export const HTTP_ACTION_REQUEST_VERSIONED =
	createVersionedDataHandler<v1.HttpActionRequest>({
		currentVersion: CURRENT_VERSION,
		migrations,
		serializeVersion: (data) => v1.encodeHttpActionRequest(data),
		deserializeVersion: (bytes) => v1.decodeHttpActionRequest(bytes),
	});

export const HTTP_ACTION_RESPONSE_VERSIONED =
	createVersionedDataHandler<v1.HttpActionResponse>({
		currentVersion: CURRENT_VERSION,
		migrations,
		serializeVersion: (data) => v1.encodeHttpActionResponse(data),
		deserializeVersion: (bytes) => v1.decodeHttpActionResponse(bytes),
	});

export const HTTP_RESPONSE_ERROR_VERSIONED =
	createVersionedDataHandler<v1.HttpResponseError>({
		currentVersion: CURRENT_VERSION,
		migrations,
		serializeVersion: (data) => v1.encodeHttpResponseError(data),
		deserializeVersion: (bytes) => v1.decodeHttpResponseError(bytes),
	});

export const HTTP_RESOLVE_REQUEST_VERSIONED =
	createVersionedDataHandler<v1.HttpResolveRequest>({
		currentVersion: CURRENT_VERSION,
		migrations,
		serializeVersion: (_) => new Uint8Array(),
		deserializeVersion: (bytes) => null,
	});

export const HTTP_RESOLVE_RESPONSE_VERSIONED =
	createVersionedDataHandler<v1.HttpResolveResponse>({
		currentVersion: CURRENT_VERSION,
		migrations,
		serializeVersion: (data) => v1.encodeHttpResolveResponse(data),
		deserializeVersion: (bytes) => v1.decodeHttpResolveResponse(bytes),
	});
