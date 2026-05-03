import { registry } from "./index.ts";
import { serve } from "@hono/node-server";
import { Hono } from "hono";
import type { Server as HttpServer } from "node:http";
import * as v8 from "node:v8";

const app = new Hono();
const port = Number.parseInt(process.env.PORT ?? "3000", 10);

process.on("exit", (code) => {
	console.log(JSON.stringify({ kind: "process_exit", code, pid: process.pid }));
});
if (process.env.SQLITE_MEMORY_SOAK_DIAGNOSTICS === "1") {
	for (const signal of ["SIGINT", "SIGTERM"] as const) {
		process.on(signal, () => {
			console.log(
				JSON.stringify({
					kind: "process_signal",
					signal,
					pid: process.pid,
					ppid: process.ppid,
					timestamp: new Date().toISOString(),
				}),
			);
			process.exit(signal === "SIGINT" ? 130 : 143);
		});
	}
}
process.on("beforeExit", (code) => {
	console.log(JSON.stringify({ kind: "process_before_exit", code, pid: process.pid }));
});
process.on("uncaughtException", (error) => {
	console.error(
		JSON.stringify({
			kind: "uncaught_exception",
			error: error.stack ?? error.message,
		}),
	);
});
process.on("unhandledRejection", (reason) => {
	console.error(
		JSON.stringify({
			kind: "unhandled_rejection",
			error: reason instanceof Error ? reason.stack ?? reason.message : String(reason),
		}),
	);
});

async function memoryBreakdown(forceGc: boolean) {
	const gc = (globalThis as typeof globalThis & { gc?: () => void }).gc;
	if (forceGc && typeof gc === "function") gc();

	const memory = process.memoryUsage();
	const heap = v8.getHeapStatistics();
	const spaces = v8.getHeapSpaceStatistics();
	const nativeNonV8Estimate = Math.max(0, memory.rss - heap.total_heap_size);
	const registryDiagnostics = await registry.diagnostics().catch((error: unknown) => ({
		error: error instanceof Error ? error.message : String(error),
	}));

	return {
		pid: process.pid,
		timestamp: new Date().toISOString(),
		uptimeSeconds: process.uptime(),
		gcRequested: forceGc,
		gcAvailable: typeof gc === "function",
		process: {
			rssBytes: memory.rss,
			heapTotalBytes: memory.heapTotal,
			heapUsedBytes: memory.heapUsed,
			externalBytes: memory.external,
			arrayBuffersBytes: memory.arrayBuffers,
		},
		v8: {
			totalHeapSizeBytes: heap.total_heap_size,
			usedHeapSizeBytes: heap.used_heap_size,
			heapSizeLimitBytes: heap.heap_size_limit,
			mallocedMemoryBytes: heap.malloced_memory,
			externalMemoryBytes: heap.external_memory,
			peakMallocedMemoryBytes: heap.peak_malloced_memory,
			spaces: spaces.map((space) => ({
				name: space.space_name,
				sizeBytes: space.space_size,
				usedBytes: space.space_used_size,
				availableBytes: space.space_available_size,
				physicalSizeBytes: space.physical_space_size,
			})),
		},
		estimates: {
			jsHeapResidentBytes: memory.heapTotal,
			jsHeapUsedBytes: memory.heapUsed,
			v8ExternalBytes: memory.external,
			nativeNonV8ResidentEstimateBytes: nativeNonV8Estimate,
		},
		registry: registryDiagnostics,
		resourceUsage: process.resourceUsage(),
	};
}

function requestHeaders(headers: Headers) {
	const entries: Array<[string, string]> = [];
	headers.forEach((value, key) => {
		entries.push([
			key,
			key === "authorization" || key === "x-rivet-token"
				? "<redacted>"
				: value,
		]);
	});
	return Object.fromEntries(entries);
}

app.get("/debug/memory", async (c) => {
	const forceGc = c.req.query("gc") === "1";
	return c.json(await memoryBreakdown(forceGc));
});

app.post("/debug/heap-snapshot", (c) => {
	if (process.env.SQLITE_MEMORY_SOAK_DIAGNOSTICS !== "1") {
		return c.json({ error: "disabled" }, 404);
	}

	const path = c.req.query("path");
	if (!path) {
		return c.json({ error: "missing path" }, 400);
	}

	const writtenPath = v8.writeHeapSnapshot(path);
	return c.json({ path: writtenPath });
});

app.use("*", async (c, next) => {
	const startedAt = Date.now();
	await next();
	console.log(
		JSON.stringify({
			kind: "request",
			method: c.req.method,
			path: new URL(c.req.url).pathname,
			headers: requestHeaders(c.req.raw.headers),
			status: c.res.status,
			durationMs: Date.now() - startedAt,
		}),
	);
});

app.all("/api/rivet/*", (c) => registry.handler(c.req.raw));
app.all("/api/rivet", (c) => registry.handler(c.req.raw));

const server = serve({ fetch: app.fetch, port }, () => {
	console.log(
		`serverless RivetKit listening on http://127.0.0.1:${port}/api/rivet`,
	);
});
const httpServer = server as unknown as HttpServer;
httpServer.requestTimeout = 0;
httpServer.headersTimeout = 0;
httpServer.keepAliveTimeout = 0;
httpServer.timeout = 0;
