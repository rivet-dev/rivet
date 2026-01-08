import invariant from "invariant";
import { InvalidRequest } from "@/actor/errors";
import { createRouter } from "@/utils/router";
import { handleHealthRequest, handleMetadataRequest } from "@/common/router";
import { logger } from "./log";
import { ServerlessStartHeadersSchema } from "@/manager/router-schema";
import { DriverConfig } from "@/registry/config";
import { RegistryConfig } from "@/registry/config";
import { createClient } from "@/client/mod";
import { RemoteManagerDriver } from "@/remote-manager-driver/mod";
import {
	ClientConfigSchema,
	convertRegistryConfigToClientConfig,
} from "@/client/config";
import { createClientWithDriver } from "@/mod";

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
				config.serverless.clientEndpoint,
			),
		);
	});
}
