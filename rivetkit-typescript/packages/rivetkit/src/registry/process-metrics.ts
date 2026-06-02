/**
 * Node.js runtime health metrics.
 *
 * Collects JS-internal data (event loop lag, GC, heap, libuv handles,
 * event loop utilization, CPU) using Node built-ins (`node:perf_hooks`,
 * `process`, `node:v8`, `PerformanceObserver`) and pushes them across NAPI
 * into Rust-side prometheus collectors registered with
 * `rivet_metrics::REGISTRY` so they appear on the existing `/metrics`
 * endpoint.
 *
 * All data collection happens here in TypeScript. The NAPI bridge is pure
 * type marshalling and the Rust side only registers + stores the metrics.
 */
import { monitorEventLoopDelay, performance, PerformanceObserver } from "node:perf_hooks";
import { getHeapStatistics } from "node:v8";
import * as napi from "@rivetkit/rivetkit-napi";

const SCRAPE_INTERVAL_MS = 5_000;
const HEARTBEAT_INTERVAL_MS = 100;
const EVENTLOOP_DELAY_RESOLUTION_MS = 20;
const NS_PER_SECOND = 1e9;
const US_PER_SECOND = 1e6;

// V8 GC kind bitfield from Node's perf_hooks documentation. A `gc` performance
// entry's `kind` field is one of these values.
const GC_KIND_NAMES: Record<number, string> = {
	1: "minor",
	2: "major",
	4: "incremental",
	8: "weakcb",
};

interface ProcessMetricsState {
	scrapeInterval: NodeJS.Timeout;
	heartbeatInterval: NodeJS.Timeout;
	gcObserver: PerformanceObserver;
	eventLoopHistogram: ReturnType<typeof monitorEventLoopDelay>;
	lastCpuUsage: NodeJS.CpuUsage;
	lastEventLoopUtilization: ReturnType<typeof performance.eventLoopUtilization>;
}

let state: ProcessMetricsState | undefined;

export function startProcessMetrics(): void {
	if (state) {
		return;
	}

	const eventLoopHistogram = monitorEventLoopDelay({
		resolution: EVENTLOOP_DELAY_RESOLUTION_MS,
	});
	eventLoopHistogram.enable();

	const gcObserver = new PerformanceObserver((list) => {
		for (const entry of list.getEntries()) {
			const kind =
				(entry as PerformanceEntry & { detail?: { kind?: number }; kind?: number }).detail
					?.kind ??
				(entry as PerformanceEntry & { kind?: number }).kind;
			if (typeof kind !== "number") continue;
			const kindName = GC_KIND_NAMES[kind];
			if (!kindName) continue;
			// `entry.duration` is in milliseconds; convert to seconds.
			napi.jsObserveGcDuration(kindName, entry.duration / 1000);
		}
	});
	gcObserver.observe({ entryTypes: ["gc"], buffered: false });

	const lastCpuUsage = process.cpuUsage();
	const lastEventLoopUtilization = performance.eventLoopUtilization();

	const heartbeatInterval = setInterval(() => {
		napi.jsSetEventloopHeartbeatTsMs(Date.now());
	}, HEARTBEAT_INTERVAL_MS);
	heartbeatInterval.unref();

	const scrapeInterval = setInterval(() => {
		try {
			collectAndPush();
		} catch {
			// Collection errors must never bring down the process; metrics
			// are best-effort.
		}
	}, SCRAPE_INTERVAL_MS);
	scrapeInterval.unref();

	state = {
		scrapeInterval,
		heartbeatInterval,
		gcObserver,
		eventLoopHistogram,
		lastCpuUsage,
		lastEventLoopUtilization,
	};

	// Emit one snapshot immediately so freshly-scraped instances have data.
	napi.jsSetEventloopHeartbeatTsMs(Date.now());
	try {
		collectAndPush();
	} catch {
		// As above; best-effort.
	}
}

export function stopProcessMetrics(): void {
	if (!state) {
		return;
	}
	clearInterval(state.scrapeInterval);
	clearInterval(state.heartbeatInterval);
	state.gcObserver.disconnect();
	state.eventLoopHistogram.disable();
	state = undefined;
}

function collectAndPush(): void {
	if (!state) return;

	// Event loop delay quantiles. `monitorEventLoopDelay()` reports values in
	// nanoseconds; convert to seconds. Reset after reading so the next window
	// reflects only the new interval.
	const hist = state.eventLoopHistogram;
	napi.jsSetEventloopLagQuantile("p50", hist.percentile(50) / NS_PER_SECOND);
	napi.jsSetEventloopLagQuantile("p90", hist.percentile(90) / NS_PER_SECOND);
	napi.jsSetEventloopLagQuantile("p99", hist.percentile(99) / NS_PER_SECOND);
	napi.jsSetEventloopLagQuantile("max", hist.max / NS_PER_SECOND);
	hist.reset();

	// Event loop utilization delta over the scrape window.
	const nextElu = performance.eventLoopUtilization();
	const eluDelta = performance.eventLoopUtilization(nextElu, state.lastEventLoopUtilization);
	state.lastEventLoopUtilization = nextElu;
	napi.jsSetEventloopUtilization(eluDelta.utilization);

	// CPU usage delta. `process.cpuUsage()` returns microseconds.
	const nextCpu = process.cpuUsage();
	const userDeltaUs = nextCpu.user - state.lastCpuUsage.user;
	const systemDeltaUs = nextCpu.system - state.lastCpuUsage.system;
	state.lastCpuUsage = nextCpu;
	if (userDeltaUs > 0) {
		napi.jsAddProcessCpuSeconds("user", userDeltaUs / US_PER_SECOND);
	}
	if (systemDeltaUs > 0) {
		napi.jsAddProcessCpuSeconds("system", systemDeltaUs / US_PER_SECOND);
	}

	// Memory + heap.
	const mem = process.memoryUsage();
	napi.jsSetProcessResidentMemoryBytes(mem.rss);
	napi.jsSetHeapBytes("used", mem.heapUsed);
	napi.jsSetHeapBytes("total", mem.heapTotal);
	const heapLimit = getHeapStatistics().heap_size_limit;
	napi.jsSetHeapBytes("limit", heapLimit);

	// libuv active handles + requests. These are unstable Node internals
	// guarded behind underscore-prefixed names; if a future Node release
	// removes them the try/catch above keeps the rest of the collection
	// alive.
	const proc = process as unknown as {
		_getActiveHandles?: () => unknown[];
		_getActiveRequests?: () => unknown[];
	};
	if (typeof proc._getActiveHandles === "function") {
		napi.jsSetActiveHandles(proc._getActiveHandles().length);
	}
	if (typeof proc._getActiveRequests === "function") {
		napi.jsSetActiveRequests(proc._getActiveRequests().length);
	}
}
