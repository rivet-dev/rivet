import {
	getNodeChildProcess,
	getNodeFs,
	getNodeFsSync,
	getNodePath,
	importNodeDependencies,
} from "@/utils/node";
import { logger } from "./log";
import { ENGINE_ENDPOINT, ENGINE_PORT } from "./constants";

export { ENGINE_ENDPOINT, ENGINE_PORT };

interface EnsureEngineProcessOptions {
	version: string;
}

async function ensureDirectoryExists(pathname: string): Promise<void> {
	const fs = await getNodeFs();
	await fs.mkdir(pathname, { recursive: true });
}

function getStoragePath(): string {
	const path = getNodePath();
	const home = process.env.HOME ?? process.cwd();
	return path.join(process.env.RIVETKIT_STORAGE_PATH ?? home, ".rivetkit");
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
	const varDir = path.join(storageRoot, "var");
	const logsDir = path.join(varDir, "logs", "rivet-engine");
	await ensureDirectoryExists(varDir);
	await ensureDirectoryExists(logsDir);

	// Check if the engine is already running on the port before resolving the binary.
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

	// Resolve the engine binary via the @rivetkit/engine-cli meta package.
	// It returns an absolute path to the rivet-engine binary shipped in a
	// platform-specific optionalDependency.
	const binaryPath = await resolveEngineBinary();

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
		cwd: storageRoot,
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
		cwd: storageRoot,
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
				issues: "https://github.com/rivet-dev/rivet/issues",
				support: "https://rivet.dev/discord",
			});
		} else if (
			stderrOutput.includes(
				"Rivet Engine has been rolled back to a previous version",
			)
		) {
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
				issues: "https://github.com/rivet-dev/rivet/issues",
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

async function resolveEngineBinary(): Promise<string> {
	// Use createRequire so TypeScript/ESM output can still load the CJS
	// engine-cli module from user install-time node_modules.
	const { createRequire } = await import("node:module");
	const require = createRequire(import.meta.url);
	let engineCli: { getEnginePath: () => string };
	try {
		engineCli = require("@rivetkit/engine-cli");
	} catch (err) {
		throw new Error(
			"@rivetkit/engine-cli is not installed — rivetkit cannot locate the rivet-engine binary. " +
				"This is a packaging bug; please report at https://github.com/rivet-dev/rivet/issues. " +
				`Underlying error: ${(err as Error).message}`,
		);
	}
	const binaryPath = engineCli.getEnginePath();
	logger().debug({ msg: "resolved engine binary", path: binaryPath });
	// Ensure executable bit (platform packages ship files; some package
	// managers don't preserve the mode on the binary).
	if (process.platform !== "win32") {
		try {
			const fs = getNodeFs();
			await fs.chmod(binaryPath, 0o755);
		} catch (err) {
			logger().warn({
				msg: "could not chmod engine binary; attempting to run anyway",
				path: binaryPath,
				err,
			});
		}
	}
	return binaryPath;
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
