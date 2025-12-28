import type { Hono } from "hono";
import { detectRuntime, stringifyError } from "../utils";
import { logger } from "./log";
import type { RunnerConfig } from "./run-config";

export async function crossPlatformServe(
	runConfig: RunnerConfig,
	app: Hono<any>,
): Promise<{ upgradeWebSocket: any }> {
	const runtime = detectRuntime();
	logger().debug({ msg: "detected runtime for serve", runtime });

	switch (runtime) {
		case "deno":
			return serveDeno(runConfig, app);
		case "bun":
			return serveBun(runConfig, app);
		case "node":
			return serveNode(runConfig, app);
		default:
			return serveNode(runConfig, app);
	}
}

async function serveNode(
	runConfig: RunnerConfig,
	app: Hono<any>,
): Promise<{ upgradeWebSocket: any }> {
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
		logger().error({
			msg: "failed to import @hono/node-server. please run 'npm install @hono/node-server @hono/node-ws'",
			error: stringifyError(err),
		});
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
		logger().error({
			msg: "failed to import @hono/node-ws. please run 'npm install @hono/node-server @hono/node-ws'",
			error: stringifyError(err),
		});
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

async function serveDeno(
	runConfig: RunnerConfig,
	app: Hono<any>,
): Promise<{ upgradeWebSocket: any }> {
	// Import hono/deno using string variable to prevent static analysis
	const honoDenoModule = "hono/deno";
	let upgradeWebSocket: any;
	try {
		const dep = await import(
			/* webpackIgnore: true */
			honoDenoModule
		);
		upgradeWebSocket = dep.upgradeWebSocket;
	} catch (err) {
		logger().error({
			msg: "failed to import hono/deno",
			error: stringifyError(err),
		});
		process.exit(1);
	}

	const port = runConfig.defaultServerPort;

	// Use Deno.serve
	// @ts-expect-error - Deno global
	Deno.serve({ port }, app.fetch);
	logger().info({ msg: "server listening", port });

	return { upgradeWebSocket };
}

async function serveBun(
	runConfig: RunnerConfig,
	app: Hono<any>,
): Promise<{ upgradeWebSocket: any }> {
	// Import hono/bun using string variable to prevent static analysis
	const honoBunModule = "hono/bun";
	let createBunWebSocket: any;
	try {
		const dep = await import(
			/* webpackIgnore: true */
			honoBunModule
		);
		createBunWebSocket = dep.createBunWebSocket;
	} catch (err) {
		logger().error({
			msg: "failed to import hono/bun",
			error: stringifyError(err),
		});
		process.exit(1);
	}

	const { websocket, upgradeWebSocket } = createBunWebSocket();

	const port = runConfig.defaultServerPort;

	// Use Bun.serve
	// @ts-expect-error - Bun global
	Bun.serve({
		fetch: app.fetch,
		port,
		websocket,
	});
	logger().info({ msg: "server listening", port });

	return { upgradeWebSocket };
}
