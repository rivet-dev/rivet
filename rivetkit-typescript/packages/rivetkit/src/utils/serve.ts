import type { Hono } from "hono";
import type { RegistryConfig } from "@/registry/config";
import { logger } from "@/registry/log";
import { detectRuntime, type Runtime, stringifyError } from "../utils";

export type ServeStatic =
	typeof import("@hono/node-server/serve-static").serveStatic;
const serveStaticLoaderPromises: Partial<
	Record<Runtime, Promise<ServeStatic>>
> = {};

export async function crossPlatformServe(
	config: RegistryConfig,
	httpPort: number,
	app: Hono<any>,
	runtime: Runtime = detectRuntime(),
): Promise<{ upgradeWebSocket: any; closeServer?: () => void }> {
	logger().debug({ msg: "detected runtime for serve", runtime });

	switch (runtime) {
		case "deno":
			return serveDeno(config, httpPort, app);
		case "bun":
			return serveBun(config, httpPort, app);
		case "node":
			return serveNode(config, httpPort, app);
		default:
			return serveNode(config, httpPort, app);
	}
}

export async function loadRuntimeServeStatic(
	runtime: Runtime,
): Promise<ServeStatic> {
	if (!serveStaticLoaderPromises[runtime]) {
		if (runtime === "node") {
			const nodeServeStaticModule = "@hono/node-server/serve-static";
			serveStaticLoaderPromises[runtime] = import(
				/* webpackIgnore: true */
				nodeServeStaticModule
			).then((x) => x.serveStatic);
		} else if (runtime === "bun") {
			const bunModule = "hono/bun";
			serveStaticLoaderPromises[runtime] = import(
				/* webpackIgnore: true */
				bunModule
			).then((x) => x.serveStatic as ServeStatic);
		} else if (runtime === "deno") {
			const denoModule = "hono/deno";
			serveStaticLoaderPromises[runtime] = import(
				/* webpackIgnore: true */
				denoModule
			).then((x) => x.serveStatic as ServeStatic);
		} else {
			throw new Error(`unsupported runtime: ${runtime}`);
		}
	}

	return await serveStaticLoaderPromises[runtime]!;
}

async function serveNode(
	config: RegistryConfig,
	httpPort: number,
	app: Hono<any>,
): Promise<{ upgradeWebSocket: any; closeServer: () => void }> {
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
	const port = httpPort;
	const hostname = config.httpHost;
	const server = serve({ fetch: app.fetch, port, hostname }, () =>
		logger().info({ msg: "server listening", port, hostname }),
	);
	injectWebSocket(server);

	const closeServer = () => {
		server.close();
	};

	return { upgradeWebSocket, closeServer };
}

async function serveDeno(
	config: RegistryConfig,
	httpPort: number,
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

	const port = httpPort;
	const hostname = config.httpHost;

	// Use Deno.serve
	Deno.serve({ port, hostname }, app.fetch);
	logger().info({ msg: "server listening", port, hostname });

	return { upgradeWebSocket };
}

async function serveBun(
	config: RegistryConfig,
	httpPort: number,
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

	const port = httpPort;
	const hostname = config.httpHost;

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
