import { createRequire } from "node:module";
import { arch, platform } from "node:process";

type NativeBinding = {
	NativeDatabase: new (bytes?: Uint8Array) => unknown;
};

const require = createRequire(import.meta.url);

const PLATFORM_ARCH = `${platform}-${arch}`;
const CANDIDATES = [
	`@rivetkit/sqlite-vfs-${PLATFORM_ARCH}`,
];

export function loadNativeBinding(): NativeBinding {
	for (const candidate of CANDIDATES) {
		try {
			return require(candidate) as NativeBinding;
		} catch (error) {
			const err = error as NodeJS.ErrnoException;
			if (err.code !== "MODULE_NOT_FOUND") {
				throw error;
			}
		}
	}

	throw new Error(
		`native sqlite-vfs addon not found for ${platform}-${arch}`,
	);
}
