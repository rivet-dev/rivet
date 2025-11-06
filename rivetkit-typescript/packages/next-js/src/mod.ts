import { existsSync, statSync } from "node:fs";
import { join } from "node:path";
import type { Registry, RunConfigInput } from "rivetkit";
import { stringifyError } from "rivetkit/utils";
import { logger } from "./log";

export const toNextHandler = (
	registry: Registry<any>,
	inputConfig: RunConfigInput = {},
) => {
	// Don't run server locally since we're using the fetch handler directly
	inputConfig.disableDefaultServer = true;

	// Configure serverless
	inputConfig.runnerKind = "serverless";

	if (process.env.NODE_ENV !== "production") {
		// Auto-configure serverless runner if not in prod
		logger().debug(
			"detected development environment, auto-starting engine and auto-configuring serverless",
		);

		const publicUrl =
			process.env.NEXT_PUBLIC_SITE_URL ??
			process.env.NEXT_PUBLIC_VERCEL_URL ??
			`http://127.0.0.1:${process.env.PORT ?? 3000}`;

		inputConfig.runEngine = true;
		inputConfig.autoConfigureServerless = {
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
	inputConfig.noWelcome = true;

	const { fetch } = registry.start(inputConfig);

	// Function that Next will call when handling requests
	const fetchWrapper = async (
		request: Request,
		{ params }: { params: Promise<{ all: string[] }> },
	): Promise<Response> => {
		const { all } = await params;

		const newUrl = new URL(request.url);
		newUrl.pathname = all.join("/");

		if (process.env.NODE_ENV !== "development") {
			// Handle request
			const newReq = new Request(newUrl, request);
			return await fetch(newReq);
		} else {
			// Special request handling for file watching
			return await handleRequestWithFileWatcher(request, newUrl, fetch);
		}
	};

	return {
		GET: fetchWrapper,
		POST: fetchWrapper,
		PUT: fetchWrapper,
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
	request: Request,
	newUrl: URL,
	fetch: (request: Request, ...args: any) => Response | Promise<Response>,
): Promise<Response> {
	// Create a new abort controller that we can abort, since the signal on
	// the request we cannot control
	const mergedController = new AbortController();
	const abortMerged = () => mergedController.abort();
	request.signal?.addEventListener("abort", abortMerged);

	// Watch for file changes in dev
	//
	// We spawn one watcher per-request since there is not a clean way of
	// cleaning up global watchers when hot reloading in Next
	const watchIntervalId = watchRouteFile(mergedController);

	// Clear interval if request is aborted
	request.signal.addEventListener("abort", () => {
		logger().debug("clearing file watcher interval: request aborted");
		clearInterval(watchIntervalId);
	});

	// Replace URL and abort signal
	const newReq = new Request(newUrl, {
		// Copy old request properties
		method: request.method,
		headers: request.headers,
		body: request.body,
		credentials: request.credentials,
		cache: request.cache,
		redirect: request.redirect,
		referrer: request.referrer,
		integrity: request.integrity,
		// Override with new signal
		signal: mergedController.signal,
		// Required for streaming body
		duplex: "half",
	} as RequestInit);

	// Handle request
	const response = await fetch(newReq);

	// HACK: Next.js does not provide a way to detect when a request
	// finishes, so we need to tap the response stream
	//
	// We can't just wait for `await fetch` to finish since SSE streams run
	// for longer
	if (response.body) {
		const wrappedStream = waitForStreamFinish(response.body, () => {
			logger().debug("clearing file watcher interval: stream finished");
			clearInterval(watchIntervalId);
		});
		return new Response(wrappedStream, {
			status: response.status,
			statusText: response.statusText,
			headers: response.headers,
		});
	} else {
		// No response body, clear interval immediately
		logger().debug("clearing file watcher interval: no response body");
		clearInterval(watchIntervalId);
		return response;
	}
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

	const routePath = join(
		process.cwd(),
		".next/server/app/api/rivet/[...all]/route.js",
	);

	let lastMtime: number | null = null;
	const checkFile = () => {
		logger().debug({ msg: "checking for file changes", routePath });
		try {
			if (!existsSync(routePath)) {
				return;
			}

			const stats = statSync(routePath);
			const mtime = stats.mtimeMs;

			if (lastMtime !== null && mtime !== lastMtime) {
				logger().info({ msg: "route file changed", routePath });
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

	return setInterval(checkFile, 1000);
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
