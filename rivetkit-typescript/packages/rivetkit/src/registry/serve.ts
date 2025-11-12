import type { Hono } from "hono";
import { logger } from "./log";
import type { RunnerConfig } from "./run-config";

export async function crossPlatformServe(
	runConfig: RunnerConfig,
	app: Hono<any>,
) {
	// Import @hono/node-server using string variable to prevent static analysis
	const nodeServerModule = "@hono/node-server";
	let serve: any;
	try {
		const dep = await import(
			/* webpackIgnore: true */
			nodeServerModule
		);
		serve = dep.serve;
	} catch (err) {
		logger().error(
			"failed to import @hono/node-server. please run 'npm install @hono/node-server @hono/node-ws'",
		);
		process.exit(1);
	}

	// Import @hono/node-ws using string variable to prevent static analysis
	const nodeWsModule = "@hono/node-ws";
	let createNodeWebSocket: any;
	try {
		const dep = await import(
			/* webpackIgnore: true */
			nodeWsModule
		);
		createNodeWebSocket = dep.createNodeWebSocket;
	} catch (err) {
		logger().error(
			"failed to import @hono/node-ws. please run 'npm install @hono/node-server @hono/node-ws'",
		);
		process.exit(1);
	}

	// Inject WS
	const { injectWebSocket, upgradeWebSocket } = createNodeWebSocket({
		app: app,
	});

	// Start server
	const port = runConfig.defaultServerPort;
	const server = serve({ fetch: app.fetch, port }, () =>
		logger().info({ msg: "server listening", port }),
	);
	injectWebSocket(server);

	return { upgradeWebSocket };
}
