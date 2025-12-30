import invariant from "invariant";
import { InvalidRequest } from "@/actor/errors";
import { createRouter } from "@/utils/router";
import { handleHealthRequest, handleMetadataRequest } from "@/common/router";
import { type RegistryConfig } from "@/registry/config/registry";
import { logger } from "./log";
import { ServerlessStartHeadersSchema } from "@/manager/router-schema";
import { DriverConfig } from "@/registry/config/base";
import { RunnerConfig } from "@/registry/config/runner";
import { ServerlessConfig } from "@/registry/config/serverless";
import { createClient } from "@/client/mod";
import { RemoteManagerDriver } from "@/remote-manager-driver/mod";
import { ClientConfigSchema } from "@/client/config";

export function buildServerlessRouter(
	driverConfig: DriverConfig,
	registryConfig: RegistryConfig,
	serverlessConfig: ServerlessConfig,
) {
	return createRouter(serverlessConfig.basePath, (router) => {
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

			// Convert config to runner config
			const newRunnerConfig: RunnerConfig = {
				...serverlessConfig,
				endpoint: endpoint,
				namespace: namespace,
				token: token,
				totalSlots: totalSlots,
				runnerName: runnerName,
				// Not supported on serverless
				runnerKey: undefined,
			};

			// Create manager driver on demand based on the properties provided
			// by headers
			const managerDriver = new RemoteManagerDriver(
				ClientConfigSchema.parse({
					endpoint,
					namespace,
					token,
					runnerName,
					headers: serverlessConfig.headers,
				}),
			);

			// Build new client that actors can use based on the given
			// credentials
			const client = createClient({
				endpoint,
				namespace,
				token,
				runnerName,
			});

			// Create new actor driver with updated config
			const actorDriver = driverConfig.actor(
				registryConfig,
				newRunnerConfig,
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
				registryConfig,
				{ serverless: {} },
				serverlessConfig.advertiseEndpoint,
			),
		);
	});
}
