import { createVersionedDataHandler } from "vbare";
import * as v1 from "../../dist/schemas/file-meta/v1";

export const CURRENT_VERSION = 1;

export const FILE_META_VERSIONED = createVersionedDataHandler<v1.FileMeta>({
	deserializeVersion: (bytes, version) => {
		switch (version) {
			case 1:
				return v1.decodeFileMeta(bytes);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	serializeVersion: (data, version) => {
		switch (version) {
			case 1:
				return v1.encodeFileMeta(data as v1.FileMeta);
			default:
				throw new Error(`Unknown version ${version}`);
		}
	},
	deserializeConverters: () => [],
	serializeConverters: () => [],
});
