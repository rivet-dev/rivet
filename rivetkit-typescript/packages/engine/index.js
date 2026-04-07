import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

const PLATFORM_PACKAGES = {
	"darwin:arm64": "@rivetkit/engine-darwin-arm64",
	"darwin:x64": "@rivetkit/engine-darwin-x64",
	"linux:x64": "@rivetkit/engine-linux-x64-musl",
	"win32:x64": "@rivetkit/engine-win32-x64-gnu",
};

export function getInstalledVersion() {
	const packageJsonPath = new URL("./package.json", import.meta.url);
	const packageJson = JSON.parse(
		fs.readFileSync(packageJsonPath, "utf8"),
	);
	return packageJson.version;
}

export function getEnginePackageNameForPlatform(platform = process.platform, arch = process.arch) {
	const packageName = PLATFORM_PACKAGES[`${platform}:${arch}`];
	if (!packageName) {
		throw new Error(
			`unsupported platform for Rivet Engine npm package: ${platform}/${arch}`,
		);
	}
	return packageName;
}

export function resolveEngineBinaryFor(platform = process.platform, arch = process.arch) {
	const packageName = getEnginePackageNameForPlatform(platform, arch);
	const packageJsonPath = require.resolve(`${packageName}/package.json`);
	const packageDir = path.dirname(packageJsonPath);
	const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
	const binaryRelativePath = packageJson.rivet?.binary;

	if (!binaryRelativePath) {
		throw new Error(
			`missing rivet.binary field in ${packageName}/package.json`,
		);
	}

	const binaryPath = path.join(packageDir, binaryRelativePath);
	if (!fs.existsSync(binaryPath)) {
		throw new Error(
			`Rivet Engine binary package ${packageName} is installed but the binary is missing at ${binaryPath}`,
		);
	}

	return {
		packageName,
		packageDir,
		binaryPath,
		version: getInstalledVersion(),
	};
}

export function resolveEngineBinary() {
	return resolveEngineBinaryFor();
}
