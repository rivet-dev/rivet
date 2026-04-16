import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

type WorkloadName =
	| "1 MiB insert"
	| "10 MiB insert"
	| "hot-row update"
	| "cold read"
	| "mixed read/write";

interface WorkloadResult {
	name: WorkloadName;
	latencyMs: number;
	roundTrips: number;
}

interface BenchReport {
	capturedAt: string;
	vfsVersion: "v1";
	source: string;
	pageSizeBytes: number;
	environment: {
		benchmarkHarness: string;
		rttMs: number;
		storage: string;
		platform: string;
		release: string;
		arch: string;
		cpuModel: string;
		cpuCount: number;
		totalMemoryGiB: number;
	};
	workloads: WorkloadResult[];
}

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "../../..");
const outputPath = path.join(
	repoRoot,
	".agent/research/sqlite/v1-baseline-bench.json",
);

function parseResults(stdout: string): { pageSizeBytes: number; workloads: WorkloadResult[] } {
	const workloads: WorkloadResult[] = [];
	let pageSizeBytes = 4096;

	for (const line of stdout.split("\n")) {
		if (line.startsWith("RESULT\t")) {
			const [, rawName, rawLatency, rawRoundTrips] = line.split("\t");
			workloads.push({
				name: rawName as WorkloadName,
				latencyMs: Number.parseFloat(rawLatency),
				roundTrips: Number.parseInt(rawRoundTrips, 10),
			});
		}

		if (line.startsWith("SUMMARY\t")) {
			const fields = Object.fromEntries(
				line
					.split("\t")
					.slice(1)
					.map((field) => field.split("=") as [string, string]),
			);
			pageSizeBytes = Number.parseInt(fields.page_size_bytes ?? "4096", 10);
		}
	}

	if (workloads.length !== 5) {
		throw new Error(`expected 5 workload results, found ${workloads.length}`);
	}

	return { pageSizeBytes, workloads };
}

function cpuModel(): string {
	return os.cpus()[0]?.model ?? "unknown";
}

function buildReport(parsed: {
	pageSizeBytes: number;
	workloads: WorkloadResult[];
}): BenchReport {
	return {
		capturedAt: new Date().toISOString(),
		vfsVersion: "v1",
		source: "examples/sqlite-raw/scripts/benchmark.ts",
		pageSizeBytes: parsed.pageSizeBytes,
		environment: {
			benchmarkHarness:
				"examples/sqlite-raw wrapper over rivetkit-sqlite-native/examples/v1_baseline_bench.rs",
			rttMs: 0,
			storage: "in-memory SqliteKv benchmark driver exercising the v1 native VFS",
			platform: os.platform(),
			release: os.release(),
			arch: os.arch(),
			cpuModel: cpuModel(),
			cpuCount: os.cpus().length,
			totalMemoryGiB: Number((os.totalmem() / 1024 ** 3).toFixed(2)),
		},
		workloads: parsed.workloads,
	};
}

function main() {
	const stdout = execFileSync(
		"cargo",
		[
			"run",
			"-p",
			"rivetkit-sqlite-native",
			"--example",
			"v1_baseline_bench",
			"--quiet",
		],
		{
			cwd: repoRoot,
			encoding: "utf8",
		},
	);

	const parsed = parseResults(stdout);
	const report = buildReport(parsed);

	mkdirSync(path.dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, `${JSON.stringify(report, null, 2)}\n`);

	console.log(stdout.trim());
	console.log(`WROTE\t${outputPath}`);
}

main();
