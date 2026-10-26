/**
 * @rivetkit/engine-cli
 *
 * Platform-specific rivet-engine binary resolver. The binary itself is shipped
 * in one of several `@rivetkit/engine-cli-<platform>` packages as an
 * optionalDependency — npm only installs the one matching the current
 * `os`/`cpu`/`libc` at install time.
 *
 * Priority at resolve time:
 *   1. `RIVET_ENGINE_BINARY` or `RIVET_ENGINE_BINARY_PATH` env var (absolute path override)
 *   2. Local `rivet-engine` binary next to this package (dev builds)
 *   3. The platform-specific `@rivetkit/engine-cli-<platform>` npm package
 */
const { existsSync } = require("node:fs");
const { dirname, join } = require("node:path");

/** Returns the name of the platform-specific npm package for the current host. */
function getPlatformPackageName() {
	const { platform, arch } = process;
	switch (platform) {
		case "linux":
			// Engine is statically linked against musl, so the single musl
			// build runs on both Alpine/musl and glibc hosts. We only publish
			// the musl variant and every linux host maps to it.
			if (arch === "x64") return "@rivetkit/engine-cli-linux-x64-musl";
			if (arch === "arm64") return "@rivetkit/engine-cli-linux-arm64-musl";
			break;
		case "darwin":
			if (arch === "x64") return "@rivetkit/engine-cli-darwin-x64";
			if (arch === "arm64") return "@rivetkit/engine-cli-darwin-arm64";
			break;
	}
	return null;
}

/** The binary filename inside each platform package. */
const BINARY_NAME =
	process.platform === "win32" ? "rivet-engine.exe" : "rivet-engine";

/**
 * Returns the absolute path to the rivet-engine binary for the current host.
 * @returns {string}
 */
function getEnginePath() {
	// Preserve the JavaScript override while accepting the shared Rust override.
	const overrideName = process.env.RIVET_ENGINE_BINARY
		? "RIVET_ENGINE_BINARY"
		: "RIVET_ENGINE_BINARY_PATH";
	const overridePath = process.env[overrideName];
	if (overridePath) {
		if (!existsSync(overridePath)) {
			throw new Error(
				`${overrideName} is set to ${overridePath} but the file does not exist`,
			);
		}
		return overridePath;
	}

	// 2) Local binary next to this package (dev flow: copy from cargo target).
	const localBinary = join(__dirname, BINARY_NAME);
	if (existsSync(localBinary)) {
		return localBinary;
	}

	// 3) Platform-specific npm package.
	const platformPkg = getPlatformPackageName();
	if (!platformPkg) {
		if (process.platform === "win32" && process.arch === "x64") {
			throw new Error(
				"@rivetkit/engine-cli: Windows x64 is not yet supported via npm platform packages.\n" +
					"Set RIVET_ENGINE_BINARY to a local rivet-engine.exe binary.",
			);
		}
		throw new Error(
			`@rivetkit/engine-cli: unsupported platform ${process.platform}/${process.arch}`,
		);
	}
	let pkgJsonPath;
	try {
		pkgJsonPath = require.resolve(`${platformPkg}/package.json`);
	} catch {
		throw new Error(
			`@rivetkit/engine-cli: platform package ${platformPkg} is not installed.\n` +
				`This usually means the platform is not supported or optionalDependencies\n` +
				`were skipped during install. Try: npm install --include=optional ${platformPkg}\n` +
				`Or set RIVET_ENGINE_BINARY to a local rivet-engine binary.`,
		);
	}
	return join(dirname(pkgJsonPath), BINARY_NAME);
}

module.exports.getEnginePath = getEnginePath;
module.exports.getPlatformPackageName = getPlatformPackageName;
