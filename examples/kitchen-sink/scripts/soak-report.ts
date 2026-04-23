// Re-analyze an existing soak JSONL produced by scripts/soak.ts.
//
// Usage:
//   pnpm tsx scripts/soak-report.ts /tmp/soak-churn-YYYYMMDD-HHMMSS-XXXX.jsonl

import { readFileSync } from "node:fs";

interface MetricPoint {
	ts: string;
	value: number | null;
}

interface MetricSeries {
	metric: string;
	labels: Record<string, string>;
	points: MetricPoint[];
}

type Mode = "churn" | "steady" | "scale";

function quantile(values: number[], q: number): number | null {
	if (values.length === 0) return null;
	const sorted = [...values].sort((a, b) => a - b);
	const idx = Math.min(sorted.length - 1, Math.floor(q * sorted.length));
	return sorted[idx];
}

function fmtPct(v: number | null): string {
	return v === null ? "n/a" : `${(v * 100).toFixed(1)}%`;
}

function linearSlopePerHour(series: MetricSeries[]): number | null {
	const points = series
		.flatMap((s) => s.points)
		.filter((p): p is { ts: string; value: number } => p.value !== null)
		.map((p) => ({ t: Date.parse(p.ts), v: p.value }))
		.filter((p) => Number.isFinite(p.t))
		.sort((a, b) => a.t - b.t);
	if (points.length < 2) return null;
	const t0 = points[0].t;
	const xs = points.map((p) => (p.t - t0) / 3_600_000);
	const ys = points.map((p) => p.v);
	const n = xs.length;
	const sumX = xs.reduce((a, b) => a + b, 0);
	const sumY = ys.reduce((a, b) => a + b, 0);
	const meanX = sumX / n;
	const meanY = sumY / n;
	let num = 0;
	let den = 0;
	for (let i = 0; i < n; i += 1) {
		num += (xs[i] - meanX) * (ys[i] - meanY);
		den += (xs[i] - meanX) ** 2;
	}
	if (den === 0) return null;
	return num / den;
}

function main(): void {
	const path = process.argv[2];
	if (!path) {
		process.stderr.write("usage: soak-report.ts <jsonl-path>\n");
		process.exit(1);
	}

	const raw = readFileSync(path, "utf8").trim().split("\n");
	const lines = raw.map((l) => JSON.parse(l) as Record<string, unknown>);

	const start = lines.find((l) => l.event === "start");
	const workloadEnd = lines.find((l) => l.event === "workload_end");
	const verdictLine = lines.find((l) => l.event === "verdict");
	const metricLines = lines.filter((l) => l.event === "metric") as unknown as MetricSeries[];
	const errorLines = lines.filter((l) => l.event === "log_error");
	const assertionFails = lines.filter((l) => l.event === "assertion_failure");

	const mode = start?.mode as Mode | undefined;
	const runId = start?.run_id as string | undefined;
	const revision = start?.revision as string | undefined;

	process.stdout.write(`run_id:    ${runId ?? "?"}\n`);
	process.stdout.write(`mode:      ${mode ?? "?"}\n`);
	process.stdout.write(`revision:  ${revision ?? "?"}\n`);

	const stats = (workloadEnd as { stats?: { cycles?: number; failures?: number } } | undefined)?.stats;
	process.stdout.write(`cycles:    ${stats?.cycles ?? 0}\n`);
	process.stdout.write(`failures:  ${stats?.failures ?? 0}\n`);
	process.stdout.write(`errors:    ${errorLines.length}\n`);
	process.stdout.write(`asserts:   ${assertionFails.length}\n`);

	const memSeries = metricLines.filter((m) => m.metric.endsWith("/memory/utilizations"));
	const cpuSeries = metricLines.filter((m) => m.metric.endsWith("/cpu/utilizations"));
	const instSeries = metricLines.filter((m) => m.metric.endsWith("/instance_count"));

	const memValues = memSeries.flatMap((s) =>
		s.points.map((p) => p.value).filter((v): v is number => v !== null),
	);
	const cpuValues = cpuSeries.flatMap((s) =>
		s.points.map((p) => p.value).filter((v): v is number => v !== null),
	);
	const instValues = instSeries.flatMap((s) =>
		s.points.map((p) => p.value).filter((v): v is number => v !== null),
	);

	const memMax = memValues.length ? Math.max(...memValues) : null;
	const memP95 = quantile(memValues, 0.95);
	const cpuMax = cpuValues.length ? Math.max(...cpuValues) : null;
	const instMax = instValues.length ? Math.max(...instValues) : null;
	const memSlope = linearSlopePerHour(memSeries);

	process.stdout.write(`memory max:    ${fmtPct(memMax)}\n`);
	process.stdout.write(`memory p95:    ${fmtPct(memP95)}\n`);
	process.stdout.write(`memory slope:  ${memSlope === null ? "n/a" : `${(memSlope * 100).toFixed(2)}%/hr`}\n`);
	process.stdout.write(`cpu max:       ${fmtPct(cpuMax)}\n`);
	process.stdout.write(`instance max:  ${instMax ?? "n/a"}\n`);

	if (verdictLine) {
		process.stdout.write(`recorded pass: ${(verdictLine as { pass?: boolean }).pass}\n`);
		const notes = (verdictLine as { notes?: string[] }).notes ?? [];
		for (const n of notes) process.stdout.write(`  note: ${n}\n`);
	}

	if (errorLines.length > 0) {
		process.stdout.write(`\nfirst 5 error log entries:\n`);
		for (const e of errorLines.slice(0, 5)) {
			const entry = (e as { entry?: { timestamp?: string; severity?: string; textPayload?: string } }).entry;
			const text = entry?.textPayload ?? JSON.stringify(entry?.textPayload ?? entry);
			process.stdout.write(`  [${entry?.severity ?? "?"}] ${entry?.timestamp ?? "?"} ${text?.slice(0, 200)}\n`);
		}
	}
}

main();
