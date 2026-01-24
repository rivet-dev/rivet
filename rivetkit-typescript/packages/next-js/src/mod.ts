import type { Registry } from "rivetkit";
import { logger } from "./log";

// Runner version set to seconds since epoch when the module loads in development mode.
//
// This creates a version number that increments each time the code is updated
// and the module reloads, allowing the engine to detect code changes via the
// /metadata endpoint and hot-reload all actors by draining older runners.
//
// We use seconds (not milliseconds) because the runner version is a u32 on the engine side.
const DEV_RUNNER_VERSION = Math.floor(Date.now() / 1000);

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
			`http://localhost:${process.env.PORT ?? 3000}`;

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

		// Set runner version to enable hot-reloading on code changes
		registry.config.runner = {
			...registry.config.runner,
			version: DEV_RUNNER_VERSION,
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
