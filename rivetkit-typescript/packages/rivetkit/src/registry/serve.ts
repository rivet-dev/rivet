import { Hono } from "hono";
import { logger } from "./log";
import type { RunnerConfig } from "./run-config";

export async function crossPlatformServe(
	runConfig: RunnerConfig,
	rivetKitRouter: Hono<any>,
	userRouter: Hono | undefined,
) {
	const app = userRouter ?? new Hono();

	// Import @hono/node-server
	let serve: any;
	try {
		const dep = await import(
			/* webpackIgnore: true */
			"@hono/node-server"
		);
		serve = dep.serve;
	} catch (err) {
		logger().error(
			"failed to import @hono/node-server. please run 'npm install @hono/node-server @hono/node-ws'",
		);
		process.exit(1);
	}

	// Mount registry
	// app.route("/registry", rivetKitRouter);
	app.route("/", rivetKitRouter);

	// Import @hono/node-ws
	let createNodeWebSocket: any;
	try {
		const dep = await import(
			/* webpackIgnore: true */
			"@hono/node-ws"
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
		app,
	});

	// Start server
	const port = runConfig.defaultServerPort;
	const server = serve({ fetch: app.fetch, port }, () =>
		logger().info({ msg: "server listening", port }),
	);
	injectWebSocket(server);

	return { upgradeWebSocket };
}
