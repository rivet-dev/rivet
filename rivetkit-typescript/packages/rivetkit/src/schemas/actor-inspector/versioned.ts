import { createVersionedDataHandler } from "vbare";

import * as v1 from "../../../dist/schemas/actor-inspector/v1";
import * as v2 from "../../../dist/schemas/actor-inspector/v2";

export const CURRENT_VERSION = 2;

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

// ToServer is identical between v1 and v2
const v1ToServerToV2 = (v1Data: v1.ToServer): v2.ToServer => {
	return v1Data as unknown as v2.ToServer;
};

const v2ToServerToV1 = (v2Data: v2.ToServer): v1.ToServer => {
	return v2Data as unknown as v1.ToServer;
};

export const TO_SERVER_VERSIONED = createVersionedDataHandler<v2.ToServer>({
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return v1.encodeToServer(data as v1.ToServer);
			case 2:
				return v2.encodeToServer(data);
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
				return v2.encodeToClient(data);
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
