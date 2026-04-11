/**
 * @rivetkit/engine-cli
 *
 * Platform-specific rivet-engine binary resolver. The binary itself is shipped
 * in one of several `@rivetkit/engine-cli-<platform>` packages as an
 * optionalDependency — npm only installs the one matching the current
 * `os`/`cpu`/`libc` at install time.
 *
 * Priority at resolve time:
 *   1. `RIVET_ENGINE_BINARY` env var (absolute path override for debugging)
 *   2. Local `rivet-engine` binary next to this package (dev builds)
 *   3. The platform-specific `@rivetkit/engine-cli-<platform>` npm package
 */
const { existsSync, readFileSync } = require("node:fs");
const { dirname, join } = require("node:path");

/** Detect if we're on Linux musl or glibc. */
function isMusl() {
	if (!process.report || typeof process.report.getReport !== "function") {
		try {
			const lddPath = require("node:child_process")
				.execSync("which ldd")
				.toString()
				.trim();
			return readFileSync(lddPath, "utf8").includes("musl");
		} catch {
			return true;
		}
	}
	const { glibcVersionRuntime } = process.report.getReport().header;
	return !glibcVersionRuntime;
}

/** Returns the name of the platform-specific npm package for the current host. */
function getPlatformPackageName() {
	const { platform, arch } = process;
	switch (platform) {
		case "linux":
			if (arch === "x64") {
				return isMusl()
					? "@rivetkit/engine-cli-linux-x64-musl"
					: "@rivetkit/engine-cli-linux-x64-gnu";
			}
			if (arch === "arm64") {
				return isMusl()
					? "@rivetkit/engine-cli-linux-arm64-musl"
					: "@rivetkit/engine-cli-linux-arm64-gnu";
			}
			break;
		case "darwin":
			if (arch === "x64") return "@rivetkit/engine-cli-darwin-x64";
			if (arch === "arm64") return "@rivetkit/engine-cli-darwin-arm64";
			break;
		case "win32":
			if (arch === "x64") return "@rivetkit/engine-cli-win32-x64";
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
	// 1) Env var override.
	if (process.env.RIVET_ENGINE_BINARY) {
		if (!existsSync(process.env.RIVET_ENGINE_BINARY)) {
			throw new Error(
				`RIVET_ENGINE_BINARY is set to ${process.env.RIVET_ENGINE_BINARY} but the file does not exist`,
			);
		}
		return process.env.RIVET_ENGINE_BINARY;
	}

	// 2) Local binary next to this package (dev flow: copy from cargo target).
	const localBinary = join(__dirname, BINARY_NAME);
	if (existsSync(localBinary)) {
		return localBinary;
	}

	// 3) Platform-specific npm package.
	const platformPkg = getPlatformPackageName();
	if (!platformPkg) {
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
