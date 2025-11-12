import { createVersionedDataHandler } from "vbare";

import * as v1 from "../../../dist/schemas/actor-inspector/v1";


export const TO_SERVER_VERSIONED = createVersionedDataHandler<v1.ToServer>({
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return v1.encodeToServer(data);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	deserializeVersion: (bytes, version) => {
		switch (version) {
			case 1:
				return v1.decodeToServer(bytes);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	deserializeConverters: () => [],
	serializeConverters: () => [],
});

export const TO_CLIENT_VERSIONED = createVersionedDataHandler<v1.ToClient>({
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return v1.encodeToClient(data);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	deserializeVersion: (bytes, version) => {
		switch (version) {
			case 1:
				return v1.decodeToClient(bytes);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	deserializeConverters: () => [],
	serializeConverters: () => [],
});
