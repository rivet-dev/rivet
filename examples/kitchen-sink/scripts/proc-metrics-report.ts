#!/usr/bin/env -S pnpm exec tsx

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

type Json = Record<string, unknown>;

interface Trace {
	name: string;
	x: number[];
	y: Array<number | null>;
	mode?: "lines" | "markers" | "lines+markers";
	type?: "scatter";
	yaxis?: "y" | "y2" | "y3";
	line?: {
		color?: string;
		dash?: string;
		shape?: "linear" | "hv";
		width?: number;
	};
	marker?: {
		color?: string;
		size?: number;
	};
}

interface Chart {
	id: string;
	title: string;
	yTitle: string;
	traces: Trace[];
	y2Title?: string;
	y3Title?: string;
}

const BYTES_PER_MIB = 1024 * 1024;
const DEFAULT_OUTPUT_ROOT = join(homedir(), "tmp/proc-metrics");
const REPO_ROOT = fileURLToPath(new URL("../../..", import.meta.url));

function usage(exitCode = 1): never {
	console.error(`Usage:
  pnpm --filter kitchen-sink proc-metrics -- <events.jsonl> [--out-dir <path>]

Examples:
  pnpm --filter kitchen-sink proc-metrics -- .agent/benchmarks/sqlite-memory-soak/no-delete-sleep-5m-10c/events.jsonl
`);
	process.exit(exitCode);
}

function readFlag(argv: string[], name: string): string | undefined {
	const index = argv.indexOf(name);
	if (index === -1) return undefined;
	const value = argv[index + 1];
	if (!value || value.startsWith("--")) usage();
	return value;
}

function sanitizeRunId(value: string): string {
	return value.replace(/[^a-zA-Z0-9_.-]/g, "_");
}

function resolveInputPath(inputPath: string): string {
	const direct = resolve(inputPath);
	if (existsSync(direct)) return direct;
	const repoRelative = resolve(REPO_ROOT, inputPath);
	if (existsSync(repoRelative)) return repoRelative;
	return direct;
}

function numberAt(value: unknown): number | null {
	return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function get(obj: unknown, path: string[]): unknown {
	let current = obj;
	for (const part of path) {
		if (typeof current !== "object" || current === null) return undefined;
		current = (current as Json)[part];
	}
	return current;
}

function bytesToMiB(value: unknown): number | null {
	const n = numberAt(value);
	return n === null ? null : n / BYTES_PER_MIB;
}

function pagesToMiB(pages: unknown, pageSize: unknown): number | null {
	const pageCount = numberAt(pages);
	const size = numberAt(pageSize) ?? 4096;
	return pageCount === null ? null : (pageCount * size) / BYTES_PER_MIB;
}

function eventTime(event: Json, firstTimestampMs: number): number {
	const elapsedMs = numberAt(event.elapsedMs);
	if (elapsedMs !== null) return elapsedMs / 1000;
	const timestamp = typeof event.timestamp === "string" ? Date.parse(event.timestamp) : NaN;
	if (Number.isFinite(timestamp)) return (timestamp - firstTimestampMs) / 1000;
	return 0;
}

function makeTrace(
	name: string,
	events: Json[],
	firstTimestampMs: number,
	read: (event: Json) => number | null,
): Trace {
	return {
		name,
		x: events.map((event) => eventTime(event, firstTimestampMs)),
		y: events.map(read),
		mode: "lines",
		type: "scatter",
	};
}

function nonEmpty(trace: Trace): boolean {
	return trace.y.some((value) => value !== null);
}

function rateTrace(
	name: string,
	events: Json[],
	firstTimestampMs: number,
	readCumulative: (event: Json) => number | null,
	scale: number,
): Trace {
	const x: number[] = [];
	const y: Array<number | null> = [];
	let prevT: number | null = null;
	let prevValue: number | null = null;
	for (const event of events) {
		const value = readCumulative(event);
		const t = eventTime(event, firstTimestampMs);
		if (value === null || prevValue === null || prevT === null || t <= prevT) {
			prevT = t;
			prevValue = value;
			continue;
		}
		x.push(t);
		y.push(((value - prevValue) / (t - prevT)) * scale);
		prevT = t;
		prevValue = value;
	}
	return { name, x, y, mode: "lines", type: "scatter" };
}

function countSeries(
	name: string,
	events: Json[],
	firstTimestampMs: number,
	match: (event: Json) => boolean,
): Trace {
	const x: number[] = [];
	const y: number[] = [];
	let count = 0;
	for (const event of events) {
		if (!match(event)) continue;
		count++;
		x.push(eventTime(event, firstTimestampMs));
		y.push(count);
	}
	return { name, x, y, mode: "lines", type: "scatter" };
}

function activeActorTrace(events: Json[], firstTimestampMs: number): Trace {
	const seen = new Set<number>();
	const points: Array<{ t: number; delta: number }> = [];
	for (const event of events) {
		const actorIndex = numberAt(event.actorIndex);
		if (actorIndex === null) continue;
		if ((event.kind === "cycle" || event.kind === "actor_reset") && !seen.has(actorIndex)) {
			seen.add(actorIndex);
			points.push({ t: eventTime(event, firstTimestampMs), delta: 1 });
		}
		if (event.kind === "actor_sleep_verified") {
			points.push({ t: eventTime(event, firstTimestampMs), delta: -1 });
		}
	}
	points.sort((a, b) => a.t - b.t);
	let active = 0;
	return {
		name: "active actor estimate",
		x: points.map((point) => point.t),
		y: points.map((point) => {
			active += point.delta;
			return active;
		}),
		mode: "lines",
		type: "scatter",
	};
}

function envoyActiveActorTrace(samples: Json[], firstTimestampMs: number): Trace {
	return {
		name: "envoy active actors",
		x: samples.map((event) => eventTime(event, firstTimestampMs)),
		y: samples.map((event) =>
			numberAt(
				get(event, [
					"kitchenSinkBreakdown",
					"registry",
					"envoyActiveActorCount",
				]),
			),
		),
		mode: "lines",
		type: "scatter",
		line: { color: "#1f7a4d", width: 2, shape: "hv" },
	};
}

function actorWakeTimes(events: Json[], firstTimestampMs: number): number[] {
	const seen = new Set<number>();
	const times: number[] = [];
	for (const event of events) {
		const actorIndex = numberAt(event.actorIndex);
		if (actorIndex === null || seen.has(actorIndex)) continue;
		if (
			event.kind === "actor_wake" ||
			event.kind === "actor_reset" ||
			event.kind === "cycle"
		) {
			seen.add(actorIndex);
			times.push(eventTime(event, firstTimestampMs));
		}
	}
	return times.sort((a, b) => a - b);
}

function addAlignmentOverlays(
	charts: Chart[],
	activeActors: Trace,
): Chart[] {
	return charts.map((chart) => {
		if (chart.id === "actors") {
			return chart;
		}
		const activeAxis: "y2" | "y3" = chart.y2Title ? "y3" : "y2";
		const activeTrace = {
			...activeActors,
			yaxis: activeAxis,
			line: {
				...activeActors.line,
				dash: "dot",
				width: 1.5,
			},
		};
		return {
			...chart,
			y2Title: chart.y2Title ?? "active actors",
			y3Title: chart.y2Title ? "active actors" : chart.y3Title,
			traces: [...chart.traces, activeTrace],
		};
	});
}

function rollingThroughputTrace(
	name: string,
	events: Json[],
	firstTimestampMs: number,
	match: (event: Json) => boolean,
	windowSeconds: number,
): Trace {
	const times = events
		.filter(match)
		.map((event) => eventTime(event, firstTimestampMs))
		.sort((a, b) => a - b);
	const x: number[] = [];
	const y: number[] = [];
	let start = 0;
	for (let end = 0; end < times.length; end += 1) {
		while (times[start] !== undefined && times[end] - times[start] > windowSeconds) {
			start++;
		}
		x.push(times[end]);
		y.push((end - start + 1) / windowSeconds);
	}
	return { name, x, y, mode: "lines", type: "scatter" };
}

function quantile(values: number[], q: number): number | null {
	if (values.length === 0) return null;
	const sorted = [...values].sort((a, b) => a - b);
	return sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))] ?? null;
}

function formatMiB(value: number | null): string {
	return value === null ? "n/a" : `${value.toFixed(1)} MiB`;
}

function htmlEscape(value: string): string {
	return value
		.replaceAll("&", "&amp;")
		.replaceAll("<", "&lt;")
		.replaceAll(">", "&gt;")
		.replaceAll('"', "&quot;");
}

function jsonForHtml(value: unknown): string {
	return JSON.stringify(value).replaceAll("<", "\\u003c");
}

function buildHtml(runId: string, inputPath: string, charts: Chart[], summary: Json): string {
	return `<!doctype html>
<html lang="en">
<head>
	<meta charset="utf-8" />
	<meta name="viewport" content="width=device-width, initial-scale=1" />
	<title>Proc Metrics ${htmlEscape(runId)}</title>
	<script src="https://cdn.plot.ly/plotly-2.35.2.min.js"></script>
	<style>
		body { margin: 0; font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background: #f7f7f4; color: #20201d; }
		header { padding: 24px 28px 10px; }
		h1 { margin: 0 0 6px; font-size: 24px; }
		code { background: #ebe9df; border-radius: 4px; padding: 2px 5px; }
		.summary { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 10px; padding: 10px 28px 18px; }
		.metric { background: #fff; border: 1px solid #dedbd0; border-radius: 6px; padding: 10px 12px; }
		.metric .label { font-size: 12px; color: #66645c; }
		.metric .value { font-size: 18px; font-weight: 650; margin-top: 2px; }
		main { padding: 0 18px 28px; display: grid; gap: 14px; }
		.chart { background: #fff; border: 1px solid #dedbd0; border-radius: 6px; min-height: 430px; }
		.note { padding: 0 28px 16px; color: #66645c; }
	</style>
</head>
<body>
	<header>
		<h1>Process Metrics: ${htmlEscape(runId)}</h1>
		<div>source: <code>${htmlEscape(inputPath)}</code></div>
	</header>
	<section class="summary" id="summary"></section>
	<div class="note">CPU and I/O charts render when the soak events include cumulative /proc samples. Older runs may only contain memory, actor, and VFS charts.</div>
	<main>
		${charts.map((chart) => `<div class="chart" id="${htmlEscape(chart.id)}"></div>`).join("\n\t\t")}
	</main>
	<script>
		const charts = ${jsonForHtml(charts)};
		const summary = ${jsonForHtml(summary)};
		const summaryEl = document.getElementById("summary");
		for (const [label, value] of Object.entries(summary)) {
			const item = document.createElement("div");
			item.className = "metric";
			item.innerHTML = '<div class="label"></div><div class="value"></div>';
			item.children[0].textContent = label;
			item.children[1].textContent = String(value);
			summaryEl.appendChild(item);
		}
		for (const chart of charts) {
			const layout = {
				title: { text: chart.title, x: 0.03 },
				paper_bgcolor: "#fff",
				plot_bgcolor: "#fff",
				margin: { l: 62, r: chart.y3Title ? 92 : chart.y2Title ? 62 : 24, t: 58, b: 52 },
				xaxis: { title: "seconds since run start", zeroline: false, gridcolor: "#ece9df" },
				yaxis: { title: chart.yTitle, zeroline: false, gridcolor: "#ece9df" },
				legend: { orientation: "h", y: -0.22 },
				hovermode: "x unified",
			};
			if (chart.y2Title) {
				layout.yaxis2 = { title: chart.y2Title, overlaying: "y", side: "right", zeroline: false };
			}
			if (chart.y3Title) {
				layout.yaxis3 = { title: chart.y3Title, overlaying: "y", side: "right", anchor: "free", position: 0.94, zeroline: false };
			}
			Plotly.newPlot(chart.id, chart.traces, layout, {
				responsive: true,
				displaylogo: false,
				toImageButtonOptions: { format: "svg", filename: chart.id },
			});
		}
	</script>
</body>
</html>
`;
}

function main(): void {
	const cliArgs = process.argv.slice(2).filter((arg) => arg !== "--");
	const inputPathArg = cliArgs[0];
	if (!inputPathArg || inputPathArg.startsWith("--")) usage();
	const inputPath = resolveInputPath(inputPathArg);
	const outDirArg = readFlag(cliArgs.slice(1), "--out-dir");
	const events = readFileSync(inputPath, "utf8")
		.trim()
		.split("\n")
		.filter(Boolean)
		.map((line) => JSON.parse(line) as Json);

	if (events.length === 0) throw new Error(`no events in ${inputPath}`);
	const runStart = events.find((event) => event.kind === "run_start");
	const samples = events.filter((event) => event.kind === "memory_sample");
	const cycles = events.filter((event) => event.kind === "cycle");
	const sleeps = events.filter((event) => event.kind === "actor_sleep_verified");
	const runId = sanitizeRunId(
		(typeof runStart?.runId === "string" && runStart.runId) ||
			(typeof samples[0]?.runId === "string" && samples[0].runId) ||
			basename(dirname(inputPath)),
	);
	const firstTimestampMs = Date.parse(
		(typeof events[0]?.timestamp === "string" && events[0].timestamp) ||
			(typeof samples[0]?.timestamp === "string" && samples[0].timestamp) ||
			new Date().toISOString(),
	);
	const estimatedActiveActors = activeActorTrace(events, firstTimestampMs);
	const reportedActiveActors = envoyActiveActorTrace(samples, firstTimestampMs);
	const activeActorOverlay = nonEmpty(reportedActiveActors)
		? reportedActiveActors
		: estimatedActiveActors;
	const wakeTimes = actorWakeTimes(events, firstTimestampMs);

	const memoryChart: Chart = {
		id: "memory-rss",
		title: "Process RSS",
		yTitle: "MiB",
		traces: [
			makeTrace("harness RSS", samples, firstTimestampMs, (event) =>
				bytesToMiB(get(event, ["harness", "rssBytes"])),
			),
			makeTrace("engine RSS", samples, firstTimestampMs, (event) =>
				bytesToMiB(get(event, ["engine", "rssBytes"])),
			),
			makeTrace("kitchen RSS", samples, firstTimestampMs, (event) =>
				bytesToMiB(get(event, ["kitchenSink", "rssBytes"])),
			),
		].filter(nonEmpty),
	};
	const pssChart: Chart = {
		id: "memory-pss-anon",
		title: "PSS And Anonymous Memory",
		yTitle: "MiB",
		traces: [
			makeTrace("engine PSS", samples, firstTimestampMs, (event) =>
				bytesToMiB(get(event, ["engine", "smapsRollup", "Pss"])),
			),
			makeTrace("engine anon PSS", samples, firstTimestampMs, (event) =>
				bytesToMiB(get(event, ["engine", "smapsRollup", "Pss_Anon"])),
			),
			makeTrace("kitchen PSS", samples, firstTimestampMs, (event) =>
				bytesToMiB(get(event, ["kitchenSink", "smapsRollup", "Pss"])),
			),
			makeTrace("kitchen anon PSS", samples, firstTimestampMs, (event) =>
				bytesToMiB(get(event, ["kitchenSink", "smapsRollup", "Pss_Anon"])),
			),
		].filter(nonEmpty),
	};
	const kitchenChart: Chart = {
		id: "kitchen-v8-native",
		title: "Kitchen-Sink V8 vs Native Estimate",
		yTitle: "MiB",
		traces: [
			makeTrace("JS heap used", samples, firstTimestampMs, (event) =>
				bytesToMiB(
					get(event, [
						"kitchenSinkBreakdown",
						"estimates",
						"jsHeapUsedBytes",
					]),
				),
			),
			makeTrace("JS heap resident", samples, firstTimestampMs, (event) =>
				bytesToMiB(
					get(event, [
						"kitchenSinkBreakdown",
						"estimates",
						"jsHeapResidentBytes",
					]),
				),
			),
			makeTrace("V8 external", samples, firstTimestampMs, (event) =>
				bytesToMiB(
					get(event, [
						"kitchenSinkBreakdown",
						"estimates",
						"v8ExternalBytes",
					]),
				),
			),
			makeTrace("native non-V8 estimate", samples, firstTimestampMs, (event) =>
				bytesToMiB(
					get(event, [
						"kitchenSinkBreakdown",
						"estimates",
						"nativeNonV8ResidentEstimateBytes",
					]),
				),
			),
		].filter(nonEmpty),
	};
	const cpuChart: Chart = {
		id: "cpu",
		title: "CPU Utilization From /proc",
			yTitle: "% of one core",
			traces: [
				rateTrace(
					"harness CPU",
					samples,
					firstTimestampMs,
					(event) => numberAt(get(event, ["harness", "cpuTotalSeconds"])),
					100,
				),
				rateTrace(
					"engine CPU",
					samples,
					firstTimestampMs,
					(event) => numberAt(get(event, ["engine", "cpuTotalSeconds"])),
					100,
				),
				rateTrace(
					"kitchen CPU",
					samples,
					firstTimestampMs,
					(event) => numberAt(get(event, ["kitchenSink", "cpuTotalSeconds"])),
					100,
				),
			].filter(nonEmpty),
		};
	const ioChart: Chart = {
		id: "io",
			title: "Process I/O Throughput",
			yTitle: "MiB/s",
			traces: [
				rateTrace(
					"engine read",
					samples,
					firstTimestampMs,
					(event) => bytesToMiB(get(event, ["engine", "io", "readBytes"])),
					1,
				),
				rateTrace(
					"engine write",
					samples,
					firstTimestampMs,
					(event) => bytesToMiB(get(event, ["engine", "io", "writeBytes"])),
					1,
				),
				rateTrace(
					"kitchen read",
					samples,
					firstTimestampMs,
					(event) =>
						bytesToMiB(get(event, ["kitchenSink", "io", "readBytes"])),
					1,
				),
				rateTrace(
					"kitchen write",
					samples,
					firstTimestampMs,
					(event) =>
						bytesToMiB(get(event, ["kitchenSink", "io", "writeBytes"])),
					1,
				),
			].filter(nonEmpty),
		};
	const threadsFdsChart: Chart = {
		id: "threads-fds",
		title: "Threads And File Descriptors",
		yTitle: "count",
		traces: [
			makeTrace("engine threads", samples, firstTimestampMs, (event) =>
				numberAt(get(event, ["engine", "threads"])),
			),
			makeTrace("engine fds", samples, firstTimestampMs, (event) =>
				numberAt(get(event, ["engine", "openFds"])),
			),
			makeTrace("kitchen threads", samples, firstTimestampMs, (event) =>
				numberAt(get(event, ["kitchenSink", "threads"])),
			),
			makeTrace("kitchen fds", samples, firstTimestampMs, (event) =>
				numberAt(get(event, ["kitchenSink", "openFds"])),
			),
		].filter(nonEmpty),
	};
	const actorChart: Chart = {
		id: "actors",
		title: "Actor Churn",
		yTitle: "count",
		traces: [
			activeActorOverlay,
			countSeries("actors slept", events, firstTimestampMs, (event) =>
				event.kind === "actor_sleep_verified",
			),
			countSeries("cycles completed", events, firstTimestampMs, (event) =>
				event.kind === "cycle",
			),
		].filter(nonEmpty),
	};
	const cycleChart: Chart = {
		id: "cycle-throughput-latency",
		title: "Cycle Throughput And Latency",
		yTitle: "cycles/s",
		y2Title: "ms",
		traces: [
			rollingThroughputTrace(
				"cycle throughput, 10s window",
				events,
				firstTimestampMs,
				(event) => event.kind === "cycle",
				10,
			),
			{
					name: "cycle latency",
					x: cycles.map((event) => eventTime(event, firstTimestampMs)),
					y: cycles.map((event) => numberAt(event.durationMs)),
					mode: "markers" as const,
					type: "scatter" as const,
					yaxis: "y2" as const,
				},
		].filter(nonEmpty),
	};
	const vfsChart: Chart = {
		id: "sqlite-vfs",
		title: "SQLite VFS Per-Cycle Metrics",
		yTitle: "MiB",
		y2Title: "pages / entries",
		traces: [
			makeTrace("page cache", cycles, firstTimestampMs, (event) =>
				pagesToMiB(
					get(event, [
						"result",
						"storage",
						"vfs",
						"pageCacheWeightedSize",
					]),
					get(event, ["result", "storage", "page_size"]),
				),
			),
			makeTrace("db size", cycles, firstTimestampMs, (event) =>
				pagesToMiB(
					get(event, ["result", "storage", "vfs", "dbSizePages"]),
					get(event, ["result", "storage", "page_size"]),
				),
			),
			{
				...makeTrace("cache entries", cycles, firstTimestampMs, (event) =>
					numberAt(
						get(event, [
							"result",
							"storage",
							"vfs",
							"pageCacheEntries",
						]),
					),
					),
					yaxis: "y2" as const,
				},
			{
				...makeTrace("dirty pages", cycles, firstTimestampMs, (event) =>
					numberAt(
						get(event, [
							"result",
							"storage",
							"vfs",
							"writeBufferDirtyPages",
						]),
					),
					),
					yaxis: "y2" as const,
				},
		].filter(nonEmpty),
	};

	const charts = addAlignmentOverlays([
		memoryChart,
		pssChart,
		kitchenChart,
		cpuChart,
		ioChart,
		threadsFdsChart,
		actorChart,
		cycleChart,
		vfsChart,
	].filter((chart) => chart.traces.length > 0), activeActorOverlay);

	const engineRss = samples
		.map((event) => bytesToMiB(get(event, ["engine", "rssBytes"])))
		.filter((value): value is number => value !== null);
	const kitchenRss = samples
		.map((event) => bytesToMiB(get(event, ["kitchenSink", "rssBytes"])))
		.filter((value): value is number => value !== null);
	const cycleLatencies = cycles
		.map((event) => numberAt(event.durationMs))
		.filter((value): value is number => value !== null);
	const summary: Json = {
		"samples": samples.length,
		"cycles": cycles.length,
		"actors slept": sleeps.length,
		"actor wakes": wakeTimes.length,
		"active actor source": nonEmpty(reportedActiveActors) ? "envoy" : "harness estimate",
		"engine RSS max": formatMiB(engineRss.length ? Math.max(...engineRss) : null),
		"engine RSS final": formatMiB(engineRss.at(-1) ?? null),
		"kitchen RSS max": formatMiB(kitchenRss.length ? Math.max(...kitchenRss) : null),
		"kitchen RSS final": formatMiB(kitchenRss.at(-1) ?? null),
		"cycle latency p95": `${
			quantile(cycleLatencies, 0.95)?.toFixed(1) ?? "n/a"
		} ms`,
	};

	const outputDir = resolve(outDirArg ?? join(DEFAULT_OUTPUT_ROOT, runId));
	mkdirSync(outputDir, { recursive: true });
	const html = buildHtml(runId, inputPath, charts, summary);
	const htmlPath = join(outputDir, "index.html");
	const jsonPath = join(outputDir, "charts.json");
	writeFileSync(htmlPath, html);
	writeFileSync(jsonPath, JSON.stringify({ runId, inputPath, summary, charts }, null, 2));
	console.log(`wrote ${htmlPath}`);
	console.log(`wrote ${jsonPath}`);
}

main();
