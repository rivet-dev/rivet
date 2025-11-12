import {
	createVersionedDataHandler,
	type MigrationFn,
} from "@/common/versioned-data";
import * as v1 from "../../../dist/schemas/actor-inspector/v1";

export const CURRENT_VERSION = 1;

const migrations = new Map<number, MigrationFn<any, any>>([]);

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
