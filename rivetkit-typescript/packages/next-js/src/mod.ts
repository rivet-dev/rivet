import { existsSync, statSync } from "node:fs";
import { join } from "node:path";
import type { Registry } from "rivetkit";
import { stringifyError } from "rivetkit/utils";
import { logger } from "./log";

const ROUTE_FILE = join(
	process.cwd(),
	".next/server/app/api/rivet/[...all]/route.js",
);
const WATCH_INTERVAL_MS = 500;

export const toNextHandler = (registry: Registry<any>) => {
	// Don't run server locally since we're using the fetch handler directly
	registry.config.serveManager = false;

	// Set basePath to "/" since Next.js route strips the /api/rivet prefix
	registry.config.serverless = {
		...registry.config.serverless,
		basePath: "/",
	};

	if (process.env.NODE_ENV !== "production") {
		logger().debug(
			"detected development environment, auto-starting engine and auto-configuring serverless",
		);

		const publicUrl =
			process.env.NEXT_PUBLIC_SITE_URL ??
			process.env.NEXT_PUBLIC_VERCEL_URL ??
			`http://127.0.0.1:${process.env.PORT ?? 3000}`;

		// Set these on the registry's config directly since the legacy inputConfig
		// isn't used by the serverless router
		registry.config.serverless.spawnEngine = true;
		registry.config.serverless.configureRunnerPool = {
			url: `${publicUrl}/api/rivet`,
			minRunners: 0,
			maxRunners: 100_000,
			requestLifespan: 300,
			slotsPerRunner: 1,
			metadata: { provider: "next-js" },
		};
	} else {
		logger().debug(
			"detected production environment, will not auto-start engine and auto-configure serverless",
		);
	}

	// Next logs this on every request
	registry.config.noWelcome = true;

	const fetchWrapper = async (
		request: Request,
		{ params }: { params: Promise<{ all: string[] }> },
	): Promise<Response> => {
		const { all } = await params;
		const targetUrl = new URL(request.url);
		targetUrl.pathname = `/${all.join("/")}`;

		if (process.env.NODE_ENV !== "production") {
			return await handleRequestWithFileWatcher(
				registry,
				request,
				targetUrl,
			);
		}

		return await registry.handler(new Request(targetUrl, request));
	};

	return {
		GET: fetchWrapper,
		POST: fetchWrapper,
		PUT: fetchWrapper,
		DELETE: fetchWrapper,
		PATCH: fetchWrapper,
		HEAD: fetchWrapper,
		OPTIONS: fetchWrapper,
	};
};

/**
 * Special request handler that will watch the source file to terminate this
 * request once complete.
 *
 * See docs on watchRouteFile for more information.
 */
async function handleRequestWithFileWatcher(
	registry: Registry<any>,
	request: Request,
	newUrl: URL,
): Promise<Response> {
	// Create a new abort controller that we can abort since we cannot control the
	// signal on the Request passed in by Next.js
	const mergedController = new AbortController();
	const abortMerged = () => mergedController.abort();
	request.signal.addEventListener("abort", abortMerged);

	const watchIntervalId = watchRouteFile(mergedController);
	const clearWatcher = () => clearInterval(watchIntervalId);

	// Clear interval if request is aborted
	request.signal.addEventListener("abort", () => {
		logger().debug("clearing file watcher interval: request aborted");
		clearWatcher();
	});

	const newReq = cloneRequestWithSignal(
		newUrl,
		request,
		mergedController.signal,
	);

	let response: Response;
	try {
		// Handle request with merged abort signal
		response = await registry.handler(newReq);
	} catch (err) {
		logger().warn({
			msg: "file watcher handler failed, falling back to direct handler",
			err: stringifyError(err),
		});
		clearWatcher();
		return await registry.handler(new Request(newUrl, request));
	}

	// HACK: Next.js does not provide a way to detect when a request finishes, so
	// we need to tap the response stream.
	if (response.body) {
		const wrappedStream = waitForStreamFinish(response.body, () => {
			logger().debug("clearing file watcher interval: stream finished");
			clearWatcher();
		});
		return new Response(wrappedStream, {
			status: response.status,
			statusText: response.statusText,
			headers: response.headers,
		});
	}

	logger().debug("clearing file watcher interval: no response body");
	clearWatcher();
	return response;
}

function cloneRequestWithSignal(
	newUrl: URL,
	request: Request,
	signal: AbortSignal,
): Request {
	const baseReq = new Request(newUrl, request);
	return new Request(baseReq, { signal });
}

/**
 * HACK: Watch for file changes on this route in order to shut down the runner.
 * We do this because Next.js does not terminate long-running requests on file
 * change, so we need to manually shut down the runner in order to trigger a
 * new `/start` request with the new code.
 *
 * We don't use file watchers since those are frequently buggy x-platform and
 * subject to misconfigured inotify limits.
 */
function watchRouteFile(abortController: AbortController): NodeJS.Timeout {
	logger().debug("starting file watcher");

	let lastMtime: number | undefined;
	let missingWarningShown = false;

	const checkFile = () => {
		logger().debug({
			msg: "checking for file changes",
			routePath: ROUTE_FILE,
		});

		try {
			if (!existsSync(ROUTE_FILE)) {
				if (!missingWarningShown) {
					logger().warn({
						msg: "route file missing, hot reloading disabled until it recompiles",
						routePath: ROUTE_FILE,
					});
					missingWarningShown = true;
				}
				lastMtime = undefined;
				return;
			}

			missingWarningShown = false;

			const stats = statSync(ROUTE_FILE);
			const mtime = stats.mtimeMs;

			if (lastMtime !== undefined && mtime !== lastMtime) {
				logger().info({
					msg: "route file changed",
					routePath: ROUTE_FILE,
				});
				abortController.abort();
			}

			lastMtime = mtime;
		} catch (err) {
			logger().info({
				msg: "failed to check for route file change",
				err: stringifyError(err),
			});
		}
	};

	checkFile();

	return setInterval(checkFile, WATCH_INTERVAL_MS);
}

/**
 * Waits for a stream to finish and calls onFinish on complete.
 *
 * Used for cancelling the file watcher.
 */
function waitForStreamFinish(
	body: ReadableStream<Uint8Array>,
	onFinish: () => void,
): ReadableStream {
	const reader = body.getReader();
	return new ReadableStream({
		async start(controller) {
			try {
				while (true) {
					const { done, value } = await reader.read();
					if (done) {
						logger().debug("stream completed");
						onFinish();
						controller.close();
						break;
					}
					controller.enqueue(value);
				}
			} catch (err) {
				logger().debug("stream errored");
				onFinish();
				controller.error(err);
			}
		},
		cancel() {
			logger().debug("stream cancelled");
			onFinish();
			reader.cancel();
		},
	});
}
