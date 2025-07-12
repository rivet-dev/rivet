import * as fs from "node:fs";
import * as os from "node:os";
import { join } from "node:path";
import { Readable } from "node:stream";
import type { ReadableStream } from "node:stream/web";

import * as pkgJson from "./package.json";

// dist
const BASE_PATH = join(__dirname);

const RIVET_CLI_BINARY_PATH = join(BASE_PATH, computeTargetFilename());
const RIVET_CLI_NODE_PATH = join(BASE_PATH, "..", pkgJson.bin.rivet);

const artifactUrl = (version: string, name: string) =>
	`https://releases.rivet.gg/rivet/${version}/${name}`;

const platform = os.platform();
const arch = os.arch();

function computeBinaryFilename() {
	if (platform === "linux") {
		return "rivet-x86_64-unknown-linux-musl";
	}

	if (platform === "darwin") {
		if (arch === "arm64") {
			return "rivet-aarch64-apple-darwin";
		}
		return "rivet-x86_64-apple-darwin";
	}

	if (platform === "win32") {
		return "rivet-x86_64-pc-windows-gnu.exe";
	}

	throw new Error(`unsupported platform ${process.platform}`);
}

function computeTargetFilename() {
	if (platform === "win32") {
		return "rivet-cli.exe";
	}

	return "rivet-cli";
}

function isYarn(): boolean {
	const { npm_config_user_agent } = process.env;
	if (npm_config_user_agent) {
		return /\byarn\//.test(npm_config_user_agent);
	}
	return false;
}

/**
 *
 * @see https://github.com/evanw/esbuild/blob/f4159a7b823cd5fe2217da2c30e8873d2f319667/lib/npm/node-install.ts#L171
 */
function maybeOptimizePackage(binPath: string, toPath: string): void {
	if (platform !== "win32" && !isYarn()) {
		const tempPath = join(__dirname, "bin-rivet");
		try {
			fs.linkSync(binPath, tempPath);
			fs.renameSync(tempPath, toPath);
			fs.unlinkSync(tempPath);
		} catch (e) {
			// Ignore errors here since this optimization is optional
		}
	}
}

async function download(version: string) {
	const binaryFilename = computeBinaryFilename();
	const url = artifactUrl(version, binaryFilename);

	console.log(`Downloading Rivet CLI ${version} for ${platform}-${arch}`);
	console.log(`Binary: ${binaryFilename}`);
	console.log(`URL: ${url}`);

	const response = await fetch(url);

	if (!response.ok) {
		throw new Error(`unexpected response ${response.statusText}`);
	}

	if (!response.body) {
		throw new Error("response body is not readable stream");
	}

	let resolve: (value?: unknown) => void;
	let reject: (error: unknown) => void;
	const promise = new Promise((res, rej) => {
		resolve = res;
		reject = rej;
	});

	const writeStream = fs.createWriteStream(RIVET_CLI_BINARY_PATH);

	writeStream.on("error", (error) => {
		reject(error);
	});

	writeStream.on("finish", () => {
		resolve();
	});

	Readable.fromWeb(response.body as ReadableStream).pipe(writeStream);

	return promise;
}

async function main() {
	console.log("Starting Rivet CLI installation...");

	try {
		await fs.promises.rm(RIVET_CLI_BINARY_PATH);
		console.log("Cleaned up existing binary");
	} catch {}

	await download(pkgJson.version);
	console.log("Download completed");

	maybeOptimizePackage(RIVET_CLI_BINARY_PATH, RIVET_CLI_NODE_PATH);
	console.log("Package optimization completed");

	fs.chmodSync(RIVET_CLI_NODE_PATH, 0o755);
	fs.chmodSync(RIVET_CLI_BINARY_PATH, 0o755);
	console.log("Permissions set");
}

main()
	.then(() => {
		console.log("Rivet CLI prepared successfully");
	})
	.catch((error) => {
		console.error("Failed to download and prepare Rivet CLI", error);
		process.exit(1);
	});
