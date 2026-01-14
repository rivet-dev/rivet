import invariant from "invariant";
import {
	EndpointMismatch,
	InvalidRequest,
	NamespaceMismatch,
} from "@/actor/errors";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import { handleHealthRequest, handleMetadataRequest } from "@/common/router";
import { ServerlessStartHeadersSchema } from "@/manager/router-schema";
import { createClientWithDriver } from "@/mod";
import type { DriverConfig, RegistryConfig } from "@/registry/config";
import { RemoteManagerDriver } from "@/remote-manager-driver/mod";
import { createRouter } from "@/utils/router";
import { logger } from "./log";

export function buildServerlessRouter(
	driverConfig: DriverConfig,
	config: RegistryConfig,
) {
	return createRouter(config.serverless.basePath, (router) => {
		// GET /
		router.get("/", (c) => {
			return c.text(
				"This is a RivetKit server.\n\nLearn more at https://rivetkit.org",
			);
		});

		// Serverless start endpoint
		router.get("/start", async (c) => {
			// Parse headers
			const parseResult = ServerlessStartHeadersSchema.safeParse({
				endpoint: c.req.header("x-rivet-endpoint"),
				token: c.req.header("x-rivet-token") ?? undefined,
				totalSlots: c.req.header("x-rivet-total-slots"),
				runnerName: c.req.header("x-rivet-runner-name"),
				namespace: c.req.header("x-rivet-namespace-name"),
			});
			if (!parseResult.success) {
				throw new InvalidRequest(
					parseResult.error.issues[0]?.message ??
						"invalid serverless start headers",
				);
			}
			const { endpoint, token, totalSlots, runnerName, namespace } =
				parseResult.data;

			logger().debug({
				msg: "received serverless runner start request",
				endpoint,
				totalSlots,
				runnerName,
				namespace,
			});

			// Validate endpoint and namespace match config to catch
			// misconfiguration or malicious requests.
			//
			// Only verify if namespace matches if endpoint configured since
			// configuring an endpoint indicates you want to assert the
			// incoming serverless requests.
			if (config.endpoint) {
				if (!endpointsMatch(endpoint, config.endpoint)) {
					throw new EndpointMismatch(config.endpoint, endpoint);
				}

				if (namespace !== config.namespace) {
					throw new NamespaceMismatch(config.namespace, namespace);
				}
			}

			// Convert config to runner config
			const newConfig: RegistryConfig = {
				...config,
				endpoint: endpoint,
				namespace: namespace,
				token: token,
				runner: {
					...config.runner,
					totalSlots: totalSlots,
					runnerName: runnerName,
					// Not supported on serverless
					runnerKey: undefined,
				},
			};

			// Create manager driver on demand based on the properties provided
			// by headers
			//
			// NOTE: This relies on the `newConfig.runner.runnerName` to
			// configure which runner to create actors on.
			const managerDriver = new RemoteManagerDriver(
				convertRegistryConfigToClientConfig(newConfig),
			);
			const client = createClientWithDriver(managerDriver);

			// Create new actor driver with updated config
			const actorDriver = driverConfig.actor(
				newConfig,
				managerDriver,
				client,
			);
			invariant(
				actorDriver.serverlessHandleStart,
				"missing serverlessHandleStart on ActorDriver",
			);

			return await actorDriver.serverlessHandleStart(c);
		});

		router.get("/health", (c) => handleHealthRequest(c));

		router.get("/metadata", (c) =>
			handleMetadataRequest(
				c,
				config,
				{ serverless: {} },
				config.publicEndpoint,
				config.publicNamespace,
				config.publicToken,
			),
		);
	});
}

/**
 * Normalizes a URL for comparison by extracting protocol, host, port, and pathname.
 * Normalizes 127.0.0.1 and 0.0.0.0 to localhost for consistent comparison.
 * Returns null if the URL is invalid.
 */
export function normalizeEndpointUrl(url: string): string | null {
	try {
		const parsed = new URL(url);
		// Normalize pathname by removing trailing slash (except for root)
		const pathname =
			parsed.pathname === "/" ? "/" : parsed.pathname.replace(/\/+$/, "");
		// Normalize loopback addresses to localhost
		const hostname =
			parsed.hostname === "127.0.0.1" || parsed.hostname === "0.0.0.0"
				? "localhost"
				: parsed.hostname;
		// Reconstruct host with normalized hostname and port
		const host = parsed.port ? `${hostname}:${parsed.port}` : hostname;
		// Reconstruct normalized URL with protocol, host, and pathname
		return `${parsed.protocol}//${host}${pathname}`;
	} catch {
		return null;
	}
}

/**
 * Compares two endpoint URLs after normalization.
 * Returns true if they match (same protocol, host, port, and path).
 */
export function endpointsMatch(a: string, b: string): boolean {
	const normalizedA = normalizeEndpointUrl(a);
	const normalizedB = normalizeEndpointUrl(b);
	if (normalizedA === null || normalizedB === null) {
		// If either URL is invalid, fall back to string comparison
		return a === b;
	}
	return normalizedA === normalizedB;
}
