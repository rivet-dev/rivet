import type { Registry, RunConfigInput } from "rivetkit";
import { logger } from "./log";

export const toNextHandler = (
	registry: Registry<any>,
	inputConfig: RunConfigInput = {},
) => {
	// Don't run server locally since we're using the fetch handler directly
	inputConfig.disableDefaultServer = true;

	// Configure serverless
	inputConfig.runnerKind = "serverless";

	// Auto-configure serverless runner if not in prod
	if (process.env.NODE_ENV !== "production") {
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

	const fetchWrapper = async (
		request: Request,
		{ params }: { params: Promise<{ all: string[] }> },
	) => {
		const { all } = await params;

		const newUrl = new URL(request.url);
		newUrl.pathname = all.join("/");
		const newReq = new Request(newUrl, request);

		return await fetch(newReq);
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
