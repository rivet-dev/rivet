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
import {
	monitorEventLoopDelay,
	PerformanceObserver,
	performance,
} from "node:perf_hooks";
import { getHeapStatistics } from "node:v8";
import * as napi from "@rivetkit/rivetkit-napi";

type OptionalProcessMetricsNapi = typeof napi & {
	jsObserveGcDuration?: (kind: string, durationSeconds: number) => void;
	jsSetEventloopHeartbeatTsMs?: (timestampMs: number) => void;
	jsSetEventloopLagQuantile?: (quantile: string, valueSeconds: number) => void;
	jsSetEventloopUtilization?: (utilization: number) => void;
	jsAddProcessCpuSeconds?: (mode: string, valueSeconds: number) => void;
	jsSetProcessResidentMemoryBytes?: (bytes: number) => void;
	jsSetHeapBytes?: (kind: string, bytes: number) => void;
	jsSetActiveHandles?: (count: number) => void;
	jsSetActiveRequests?: (count: number) => void;
};

type GcPerformanceEntry = {
	duration: number;
	detail?: { kind?: number };
	kind?: number;
};

const processMetricsNapi = napi as OptionalProcessMetricsNapi;

// Some napi process-metrics symbols may be missing on older native binaries
// (the auto-generated index.js destructures them as `undefined` if the
// underlying `.node` was built before they were added). Guard each call so
// the metrics collection runs as a no-op instead of throwing
// `TypeError: napi.jsXxx is not a function` on every interval tick.
function callIfFn<T extends unknown[]>(
	fn: ((...args: T) => void) | undefined,
	...args: T
): void {
	if (typeof fn === "function") {
		fn(...args);
	}
}

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
	lastEventLoopUtilization: ReturnType<
		typeof performance.eventLoopUtilization
	>;
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
			const gcEntry = entry as GcPerformanceEntry;
			const kind = gcEntry.detail?.kind ?? gcEntry.kind;
			if (typeof kind !== "number") continue;
			const kindName = GC_KIND_NAMES[kind];
			if (!kindName) continue;
			// `entry.duration` is in milliseconds; convert to seconds.
			callIfFn(
				processMetricsNapi.jsObserveGcDuration,
				kindName,
				gcEntry.duration / 1000,
			);
		}
	});
	gcObserver.observe({ entryTypes: ["gc"], buffered: false });

	const lastCpuUsage = process.cpuUsage();
	const lastEventLoopUtilization = performance.eventLoopUtilization();

	const heartbeatInterval = setInterval(() => {
		callIfFn(processMetricsNapi.jsSetEventloopHeartbeatTsMs, Date.now());
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
	callIfFn(processMetricsNapi.jsSetEventloopHeartbeatTsMs, Date.now());
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
	callIfFn(
		processMetricsNapi.jsSetEventloopLagQuantile,
		"p50",
		hist.percentile(50) / NS_PER_SECOND,
	);
	callIfFn(
		processMetricsNapi.jsSetEventloopLagQuantile,
		"p90",
		hist.percentile(90) / NS_PER_SECOND,
	);
	callIfFn(
		processMetricsNapi.jsSetEventloopLagQuantile,
		"p99",
		hist.percentile(99) / NS_PER_SECOND,
	);
	callIfFn(
		processMetricsNapi.jsSetEventloopLagQuantile,
		"max",
		hist.max / NS_PER_SECOND,
	);
	hist.reset();

	// Event loop utilization delta over the scrape window.
	const nextElu = performance.eventLoopUtilization();
	const eluDelta = performance.eventLoopUtilization(
		nextElu,
		state.lastEventLoopUtilization,
	);
	state.lastEventLoopUtilization = nextElu;
	callIfFn(processMetricsNapi.jsSetEventloopUtilization, eluDelta.utilization);

	// CPU usage delta. `process.cpuUsage()` returns microseconds.
	const nextCpu = process.cpuUsage();
	const userDeltaUs = nextCpu.user - state.lastCpuUsage.user;
	const systemDeltaUs = nextCpu.system - state.lastCpuUsage.system;
	state.lastCpuUsage = nextCpu;
	if (userDeltaUs > 0) {
		callIfFn(
			processMetricsNapi.jsAddProcessCpuSeconds,
			"user",
			userDeltaUs / US_PER_SECOND,
		);
	}
	if (systemDeltaUs > 0) {
		callIfFn(
			processMetricsNapi.jsAddProcessCpuSeconds,
			"system",
			systemDeltaUs / US_PER_SECOND,
		);
	}

	// Memory + heap.
	const mem = process.memoryUsage();
	callIfFn(processMetricsNapi.jsSetProcessResidentMemoryBytes, mem.rss);
	callIfFn(processMetricsNapi.jsSetHeapBytes, "used", mem.heapUsed);
	callIfFn(processMetricsNapi.jsSetHeapBytes, "total", mem.heapTotal);
	const heapLimit = getHeapStatistics().heap_size_limit;
	callIfFn(processMetricsNapi.jsSetHeapBytes, "limit", heapLimit);

	// libuv active handles + requests. These are unstable Node internals
	// guarded behind underscore-prefixed names; if a future Node release
	// removes them the try/catch above keeps the rest of the collection
	// alive.
	const proc = process as unknown as {
		_getActiveHandles?: () => unknown[];
		_getActiveRequests?: () => unknown[];
	};
	if (typeof proc._getActiveHandles === "function") {
		callIfFn(processMetricsNapi.jsSetActiveHandles, proc._getActiveHandles().length);
	}
	if (typeof proc._getActiveRequests === "function") {
		callIfFn(processMetricsNapi.jsSetActiveRequests, proc._getActiveRequests().length);
	}
}
