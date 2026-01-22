import type { Hono } from "hono";
import { detectRuntime, stringifyError } from "../utils";
import { logger } from "./log";
import { RegistryConfig } from "./config";

// TODO: Go back to dynamic import for this
import getPort from "get-port";

const DEFAULT_PORT = 6420;

/**
 * Finds a free port starting from the given port.
 *
 * Tries ports incrementally until a free one is found.
 */
export async function findFreePort(
	startPort: number = DEFAULT_PORT,
): Promise<number> {
	// TODO: Fix this
	// const getPortModule = "get-port";
	// const { default: getPort } = await import(/* webpackIgnore: true */ getPortModule);

	// Create an iterable of ports starting from startPort
	function* portRange(start: number, count: number = 100): Iterable<number> {
		for (let i = 0; i < count; i++) {
			yield start + i;
		}
	}

	return getPort({ port: portRange(startPort) });
}

export async function crossPlatformServe(
	config: RegistryConfig,
	managerPort: number,
	app: Hono<any>,
): Promise<{ upgradeWebSocket: any }> {
	const runtime = detectRuntime();
	logger().debug({ msg: "detected runtime for serve", runtime });

	switch (runtime) {
		case "deno":
			return serveDeno(config, managerPort, app);
		case "bun":
			return serveBun(config, managerPort, app);
		case "node":
			return serveNode(config, managerPort, app);
		default:
			return serveNode(config, managerPort, app);
	}
}

async function serveNode(
	config: RegistryConfig,
	managerPort: number,
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
	const port = managerPort;
	const hostname = config.managerHost;
	const server = serve({ fetch: app.fetch, port, hostname }, () =>
		logger().info({ msg: "server listening", port, hostname }),
	);
	injectWebSocket(server);

	return { upgradeWebSocket };
}

async function serveDeno(
	config: RegistryConfig,
	managerPort: number,
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

	const port = config.managerPort;
	const hostname = config.managerHost;

	// Use Deno.serve
	Deno.serve({ port, hostname }, app.fetch);
	logger().info({ msg: "server listening", port, hostname });

	return { upgradeWebSocket };
}

async function serveBun(
	config: RegistryConfig,
	managerPort: number,
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

	const port = config.managerPort;
	const hostname = config.managerHost;

	// Use Bun.serve
	// @ts-expect-error - Bun global
	Bun.serve({
		fetch: app.fetch,
		port,
		hostname,
		websocket,
	});
	logger().info({ msg: "server listening", port, hostname });

	return { upgradeWebSocket };
}
