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
import { ENGINE_ENDPOINT, ENGINE_PORT } from "./constants";

export { ENGINE_ENDPOINT, ENGINE_PORT };

const ENGINE_BASE_URL = "https://releases.rivet.dev/rivet";
const ENGINE_BINARY_NAME = "rivet-engine";

interface EnsureEngineProcessOptions {
	version: string;
}

export async function ensureEngineProcess(
	options: EnsureEngineProcessOptions,
): Promise<void> {
	importNodeDependencies();

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

	// Check if the engine is already running on the port before downloading
	if (await isEngineRunning()) {
		try {
			const health = await waitForEngineHealth();
			logger().debug({
				msg: "engine already running and healthy",
				version: health.version,
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

	const executableName =
		process.platform === "win32"
			? `${ENGINE_BINARY_NAME}-${options.version}.exe`
			: `${ENGINE_BINARY_NAME}-${options.version}`;
	const binaryPath = path.join(binDir, executableName);
	await downloadEngineBinaryIfNeeded(binaryPath, options.version, varDir);
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
			// Development environment overrides for Rivet Engine.
			//
			// NOTE: When modifying these env vars, also update scripts/run/dev-env.sh
			// to keep them in sync for manual engine runs.
			//
			// In development, runners can be terminated without a graceful
			// shutdown (i.e. SIGKILL instead of SIGTERM). This is treated as a
			// crash by Rivet Engine in production and implements a backoff for
			// rescheduling actors in case of a crash loop.
			//
			// This is problematic in development since this will cause actors
			// to become unresponsive if frequently killing your dev server.
			//
			// We reduce the timeouts for resetting a runner as healthy in
			// order to account for this.
			RIVET__PEGBOARD__RETRY_RESET_DURATION: "100",
			RIVET__PEGBOARD__BASE_RETRY_TIMEOUT: "100",
			// Set max exponent to 1 to have a maximum of base_retry_timeout
			RIVET__PEGBOARD__RESCHEDULE_BACKOFF_MAX_EXPONENT: "1",
			// Reduce thresholds for faster development iteration
			//
			// Default ping interval is 3s, this gives a 2s & 4s grace
			RIVET__PEGBOARD__RUNNER_ELIGIBLE_THRESHOLD: "5000",
			RIVET__PEGBOARD__RUNNER_LOST_THRESHOLD: "7000",
			// Allow faster metadata polling for hot-reload in development (in milliseconds)
			RIVET__PEGBOARD__MIN_METADATA_POLL_INTERVAL: "1000",
			// Reduce shutdown durations for faster development iteration (in seconds)
			RIVET__RUNTIME__WORKER_SHUTDOWN_DURATION: "1",
			RIVET__RUNTIME__GUARD_SHUTDOWN_DURATION: "1",
			// Force exit after this duration (must be > worker and guard shutdown durations)
			RIVET__RUNTIME__FORCE_SHUTDOWN_DURATION: "2",
		},
	});

	if (!child.pid) {
		throw new Error("failed to spawn rivet engine process");
	}

	// Pipe stdout and stderr to log files
	if (child.stdout) {
		child.stdout.pipe(stdoutStream);
	}
	// Collect stderr for error detection
	const stderrChunks: Buffer[] = [];
	if (child.stderr) {
		child.stderr.on("data", (chunk: Buffer) => {
			stderrChunks.push(chunk);
		});
		child.stderr.pipe(stderrStream);
	}
	logger().debug({
		msg: "spawned engine process",
		pid: child.pid,
		cwd: path.dirname(binaryPath),
	});

	child.once("exit", (code, signal) => {
		const stderrOutput = Buffer.concat(stderrChunks).toString("utf-8");

		// Check for specific error conditions
		if (stderrOutput.includes("LOCK: Resource temporarily unavailable")) {
			logger().error({
				msg: "another instance of rivet engine is unexpectedly running, this is an internal error",
				code,
				signal,
				stdoutLog: stdoutLogPath,
				stderrLog: stderrLogPath,
				issues: "https://github.com/rivet-dev/rivetkit/issues",
				support: "https://rivet.dev/discord",
			});
		} else if (stderrOutput.includes("Rivet Engine has been rolled back to a previous version")) {
			logger().error({
				msg: "rivet engine version downgrade detected",
				hint: `You attempted to downgrade the RivetKit version in development. To fix this, nuke the database by running: '${binaryPath}' database nuke --yes`,
				code,
				signal,
				stdoutLog: stdoutLogPath,
				stderrLog: stderrLogPath,
			});
		} else {
			logger().warn({
				msg: "engine process exited, please report this error",
				code,
				signal,
				stdoutLog: stdoutLogPath,
				stderrLog: stderrLogPath,
				issues: "https://github.com/rivet-dev/rivetkit/issues",
				support: "https://rivet.dev/discord",
			});
		}
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
	const downloadUrl = `${ENGINE_BASE_URL}/${version}/engine/${remoteFile}`;
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
			response.body as any,
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
		response = await fetch(`http://127.0.0.1:${port}/health`);
	} catch (err) {
		// Nothing is running on this port
		return false;
	}

	if (response.ok) {
		const health = (await response.json()) as EngineHealthResponse;

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

interface EngineHealthResponse {
	status?: string;
	runtime?: string;
	version?: string;
}

async function waitForEngineHealth(): Promise<EngineHealthResponse> {
	const maxRetries = Math.ceil(HEALTH_MAX_WAIT / HEALTH_INTERVAL);

	logger().debug({ msg: "waiting for engine health check" });

	for (let i = 0; i < maxRetries; i++) {
		try {
			const response = await fetch(`${ENGINE_ENDPOINT}/health`, {
				signal: AbortSignal.timeout(1000),
			});
			if (response.ok) {
				const health = (await response.json()) as EngineHealthResponse;
				logger().debug({ msg: "engine health check passed" });
				return health;
			}
		} catch (error) {
			// Expected to fail while engine is starting up
			logger().debug({ msg: "engine health check failed", error });
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
