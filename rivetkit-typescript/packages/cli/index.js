#!/usr/bin/env node
const { existsSync } = require("node:fs");
const { spawnSync } = require("node:child_process");
const { dirname, join } = require("node:path");

function getPlatformPackageName() {
	const { platform, arch } = process;
	switch (platform) {
		case "linux":
			if (arch === "x64") return "@rivetkit/cli-linux-x64-musl";
			if (arch === "arm64") return "@rivetkit/cli-linux-arm64-musl";
			break;
		case "darwin":
			if (arch === "x64") return "@rivetkit/cli-darwin-x64";
			if (arch === "arm64") return "@rivetkit/cli-darwin-arm64";
			break;
		case "win32":
			if (arch === "x64") return "@rivetkit/cli-win32-x64";
			break;
	}
	return null;
}

const BINARY_NAME = process.platform === "win32" ? "rivet.exe" : "rivet";

function getCliPath() {
	if (process.env.RIVET_CLI_BINARY) {
		if (!existsSync(process.env.RIVET_CLI_BINARY)) {
			throw new Error(
				`RIVET_CLI_BINARY is set to ${process.env.RIVET_CLI_BINARY} but the file does not exist`,
			);
		}
		return process.env.RIVET_CLI_BINARY;
	}

	const localBinary = join(__dirname, BINARY_NAME);
	if (existsSync(localBinary)) return localBinary;

	const platformPkg = getPlatformPackageName();
	if (!platformPkg) {
		throw new Error(
			`@rivetkit/cli: unsupported platform ${process.platform}/${process.arch}`,
		);
	}

	let pkgJsonPath;
	try {
		pkgJsonPath = require.resolve(`${platformPkg}/package.json`);
	} catch {
		if (process.platform === "win32" && process.arch === "x64") {
			const version = require("./package.json").version;
			if (
				typeof version === "string" &&
				version.startsWith("0.0.0-")
			) {
				throw new Error(
					"@rivetkit/cli: Windows x64 binaries are only published for release versions.\n" +
						`The current package version (${version}) is a preview build, so @rivetkit/cli-win32-x64 was intentionally not published.\n` +
						"Use a release build or set RIVET_CLI_BINARY to a local rivet.exe binary.",
				);
			}
		}
		throw new Error(
			`@rivetkit/cli: platform package ${platformPkg} is not installed.\n` +
				"Optional dependencies may have been skipped. Try npm install --include=optional @rivetkit/cli.",
		);
	}
	return join(dirname(pkgJsonPath), BINARY_NAME);
}

if (require.main === module) {
	const result = spawnSync(getCliPath(), process.argv.slice(2), {
		stdio: "inherit",
		env: process.env,
	});
	if (result.error) throw result.error;
	process.exit(result.status ?? 1);
}

module.exports.getCliPath = getCliPath;
module.exports.getPlatformPackageName = getPlatformPackageName;
