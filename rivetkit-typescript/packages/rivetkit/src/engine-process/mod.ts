import {
	ensureDirectoryExists,
	getStoragePath,
} from "@/drivers/file-system/utils";
import {
	getNodeChildProcess,
	getNodeCrypto,
	getNodeFs,
	getNodeFsSync,
	getNodePath,
	getNodeStream,
	importNodeDependencies,
} from "@/utils/node";
import { logger } from "./log";

export const ENGINE_PORT = 6420;
export const ENGINE_ENDPOINT = `http://localhost:${ENGINE_PORT}`;

const ENGINE_BASE_URL = "https://releases.rivet.gg/engine";
const ENGINE_BINARY_NAME = "rivet-engine";

interface EnsureEngineProcessOptions {
	version: string;
}

export async function ensureEngineProcess(
	options: EnsureEngineProcessOptions,
): Promise<void> {
	// Import Node.js dependencies first
	await importNodeDependencies();

	logger().debug({
		msg: "ensuring engine process",
		version: options.version,
	});

	const path = getNodePath();
	const storageRoot = getStoragePath();
	const binDir = path.join(storageRoot, "bin");
	const varDir = path.join(storageRoot, "var");
	const logsDir = path.join(varDir, "logs", "rivet-engine");
	await ensureDirectoryExists(binDir);
	await ensureDirectoryExists(varDir);
	await ensureDirectoryExists(logsDir);

	const executableName =
		process.platform === "win32"
			? `${ENGINE_BINARY_NAME}-${options.version}.exe`
			: `${ENGINE_BINARY_NAME}-${options.version}`;
	const binaryPath = path.join(binDir, executableName);
	await downloadEngineBinaryIfNeeded(binaryPath, options.version, varDir);

	// Check if the engine is already running on the port
	if (await isEngineRunning()) {
		try {
			await waitForEngineHealth();
			logger().debug({
				msg: "engine already running and healthy",
				version: options.version,
			});
			return;
		} catch (error) {
			logger().warn({
				msg: "existing engine process not healthy, cannot restart automatically",
				error,
			});
			throw new Error(
				"Engine process exists but is not healthy. Please manually stop the process on port 6420 and retry.",
			);
		}
	}
	// Create log file streams with timestamp in the filename
	const timestamp = new Date()
		.toISOString()
		.replace(/:/g, "-")
		.replace(/\./g, "-");
	const stdoutLogPath = path.join(logsDir, `engine-${timestamp}-stdout.log`);
	const stderrLogPath = path.join(logsDir, `engine-${timestamp}-stderr.log`);

	const fsSync = getNodeFsSync();
	const stdoutStream = fsSync.createWriteStream(stdoutLogPath, {
		flags: "a",
	});
	const stderrStream = fsSync.createWriteStream(stderrLogPath, {
		flags: "a",
	});

	logger().debug({
		msg: "creating engine log files",
		stdout: stdoutLogPath,
		stderr: stderrLogPath,
	});

	const childProcess = getNodeChildProcess();
	const child = childProcess.spawn(binaryPath, ["start"], {
		cwd: path.dirname(binaryPath),
		stdio: ["inherit", "pipe", "pipe"],
		env: {
			...process.env,
		},
	});

	if (!child.pid) {
		throw new Error("failed to spawn rivet engine process");
	}

	// Pipe stdout and stderr to log files
	if (child.stdout) {
		child.stdout.pipe(stdoutStream);
	}
	if (child.stderr) {
		child.stderr.pipe(stderrStream);
	}
	logger().debug({
		msg: "spawned engine process",
		pid: child.pid,
		cwd: path.dirname(binaryPath),
	});

	child.once("exit", (code, signal) => {
		logger().warn({
			msg: "engine process exited, please report this error",
			code,
			signal,
			issues: "https://github.com/rivet-dev/rivetkit/issues",
			support: "https://rivet.dev/discord",
		});
		// Clean up log streams
		stdoutStream.end();
		stderrStream.end();
	});

	child.once("error", (error) => {
		logger().error({
			msg: "engine process failed",
			error,
		});
		// Clean up log streams on error
		stdoutStream.end();
		stderrStream.end();
	});

	// Wait for engine to be ready
	await waitForEngineHealth();

	logger().info({
		msg: "engine process started",
		pid: child.pid,
		version: options.version,
		logs: {
			stdout: stdoutLogPath,
			stderr: stderrLogPath,
		},
	});
}

async function downloadEngineBinaryIfNeeded(
	binaryPath: string,
	version: string,
	varDir: string,
): Promise<void> {
	const binaryExists = await fileExists(binaryPath);
	if (binaryExists) {
		logger().debug({
			msg: "engine binary already cached",
			version,
			path: binaryPath,
		});
		return;
	}

	const { targetTriplet, extension } = resolveTargetTriplet();
	const remoteFile = `${ENGINE_BINARY_NAME}-${targetTriplet}${extension}`;
	const downloadUrl = `${ENGINE_BASE_URL}/${version}/${remoteFile}`;
	logger().info({
		msg: "downloading engine binary",
		url: downloadUrl,
		path: binaryPath,
		version,
	});

	const response = await fetch(downloadUrl);
	if (!response.ok || !response.body) {
		throw new Error(
			`failed to download rivet engine binary from ${downloadUrl}: ${response.status} ${response.statusText}`,
		);
	}

	// Generate unique temp file name to prevent parallel download conflicts
	const crypto = getNodeCrypto();
	const tempPath = `${binaryPath}.${crypto.randomUUID()}.tmp`;
	const startTime = Date.now();

	logger().debug({
		msg: "starting binary download",
		tempPath,
		contentLength: response.headers.get("content-length"),
	});

	// Warn user if download is taking a long time
	const slowDownloadWarning = setTimeout(() => {
		logger().warn({
			msg: "engine binary download is taking longer than expected, please be patient",
			version,
		});
	}, 5000);

	try {
		const stream = getNodeStream();
		const fsSync = getNodeFsSync();
		await stream.pipeline(
			response.body,
			fsSync.createWriteStream(tempPath),
		);

		// Clear the slow download warning
		clearTimeout(slowDownloadWarning);

		// Get file size to verify download
		const fs = getNodeFs();
		const stats = await fs.stat(tempPath);
		const downloadDuration = Date.now() - startTime;

		if (process.platform !== "win32") {
			await fs.chmod(tempPath, 0o755);
		}
		await fs.rename(tempPath, binaryPath);

		logger().debug({
			msg: "engine binary download complete",
			version,
			path: binaryPath,
			size: stats.size,
			durationMs: downloadDuration,
		});
		logger().info({
			msg: "engine binary downloaded",
			version,
			path: binaryPath,
		});
	} catch (error) {
		// Clear the slow download warning
		clearTimeout(slowDownloadWarning);

		// Clean up partial temp file on error
		logger().warn({
			msg: "engine download failed, please report this error",
			tempPath,
			error,
			issues: "https://github.com/rivet-dev/rivetkit/issues",
			support: "https://rivet.dev/discord",
		});
		try {
			const fs = getNodeFs();
			await fs.unlink(tempPath);
		} catch (unlinkError) {
			// Ignore errors when cleaning up (file may not exist)
		}
		throw error;
	}
}
//
function resolveTargetTriplet(): { targetTriplet: string; extension: string } {
	return resolveTargetTripletFor(process.platform, process.arch);
}

export function resolveTargetTripletFor(
	platform: NodeJS.Platform,
	arch: typeof process.arch,
): { targetTriplet: string; extension: string } {
	switch (platform) {
		case "darwin":
			if (arch === "arm64") {
				return { targetTriplet: "aarch64-apple-darwin", extension: "" };
			}
			if (arch === "x64") {
				return { targetTriplet: "x86_64-apple-darwin", extension: "" };
			}
			break;
		case "linux":
			if (arch === "x64") {
				return {
					targetTriplet: "x86_64-unknown-linux-musl",
					extension: "",
				};
			}
			break;
		case "win32":
			if (arch === "x64") {
				return {
					targetTriplet: "x86_64-pc-windows-gnu",
					extension: ".exe",
				};
			}
			break;
	}

	throw new Error(
		`unsupported platform for rivet engine binary: ${platform}/${arch}`,
	);
}
async function isEngineRunning(): Promise<boolean> {
	// Check if the engine is running on the port
	return await checkIfEngineAlreadyRunningOnPort(ENGINE_PORT);
}

async function checkIfEngineAlreadyRunningOnPort(
	port: number,
): Promise<boolean> {
	let response: Response;
	try {
		response = await fetch(`http://localhost:${port}/health`);
	} catch (err) {
		// Nothing is running on this port
		return false;
	}

	if (response.ok) {
		const health = (await response.json()) as {
			status?: string;
			runtime?: string;
			version?: string;
		};

		// Check what's running on this port
		if (health.runtime === "engine") {
			logger().debug({
				msg: "rivet engine already running on port",
				port,
			});
			return true;
		} else if (health.runtime === "rivetkit") {
			logger().error({
				msg: "another rivetkit process is already running on port",
				port,
			});
			throw new Error(
				"RivetKit process already running on port 6420, stop that process and restart this.",
			);
		} else {
			throw new Error(
				"Unknown process running on port 6420, cannot identify what it is.",
			);
		}
	}

	// Port responded but not with OK status
	return false;
}
async function fileExists(filePath: string): Promise<boolean> {
	try {
		const fs = getNodeFs();
		await fs.access(filePath);
		return true;
	} catch {
		return false;
	}
}

const HEALTH_MAX_WAIT = 10_000;
const HEALTH_INTERVAL = 100;

async function waitForEngineHealth(): Promise<void> {
	const maxRetries = Math.ceil(HEALTH_MAX_WAIT / HEALTH_INTERVAL);

	logger().debug({ msg: "waiting for engine health check" });

	for (let i = 0; i < maxRetries; i++) {
		try {
			const response = await fetch(`${ENGINE_ENDPOINT}/health`);
			if (response.ok) {
				logger().debug({ msg: "engine health check passed" });
				return;
			}
		} catch (error) {
			// Expected to fail while engine is starting up
			if (i === maxRetries - 1) {
				throw new Error(
					`engine health check failed after ${maxRetries} retries: ${error}`,
				);
			}
		}

		if (i < maxRetries - 1) {
			logger().trace({
				msg: "engine not ready, retrying",
				attempt: i + 1,
				maxRetries,
			});
			await new Promise((resolve) =>
				setTimeout(resolve, HEALTH_INTERVAL),
			);
		}
	}

	throw new Error(`engine health check failed after ${maxRetries} retries`);
}
