/**
 * `rivet-cloud logs` — stream real-time logs from a Rivet Cloud managed pool.
 *
 * The implementation mirrors the frontend's use-deployment-logs-stream.ts:
 * it opens an SSE connection to the Cloud API log endpoint, reconnects on
 * failure with exponential back-off (up to 8 retries), and prints every log
 * line to stdout.
 */

import type { Command } from "commander";
import { CloudClient } from "../lib/client.ts";
import { resolveToken } from "../lib/auth.ts";
import {
	colors,
	detail,
	fatal,
	formatRegion,
	formatTimestamp,
	header,
} from "../utils/output.ts";

export interface LogsOptions {
	token?: string;
	namespace: string;
	pool: string;
	filter?: string;
	region?: string;
	apiUrl?: string;
}

const MAX_RETRIES = 8;
const BASE_DELAY_MS = 1_000;

export function registerLogsCommand(program: Command): void {
	program
		.command("logs")
		.description("Stream real-time logs from a Rivet Cloud managed pool")
		.option("-t, --token <token>", "Cloud API token (overrides RIVET_CLOUD_TOKEN)")
		.option("-n, --namespace <name>", "Target namespace", "production")
		.option("-p, --pool <name>", "Managed pool name", "default")
		.option("--filter <text>", "Only show log lines containing this string")
		.option("--region <region>", "Filter logs by region slug")
		.option("--api-url <url>", "Cloud API base URL", "https://cloud-api.rivet.dev")
		.action(async (opts: LogsOptions) => {
			await runLogs(opts);
		});
}

async function runLogs(opts: LogsOptions): Promise<void> {
	const token = resolveToken(opts.token);
	const client = new CloudClient({ token, baseUrl: opts.apiUrl });

	// Resolve org + project from token
	let identity: { project: string; organization: string };

	try {
		identity = await client.inspect();
	} catch (err: unknown) {
		fatal(
			`Authentication failed: ${err instanceof Error ? err.message : String(err)}`,
			"Check that RIVET_CLOUD_TOKEN is valid.",
		);
	}

	const { project, organization: org } = identity;

	console.log("");
	header("Rivet Cloud Logs");
	detail("Project", project);
	detail("Namespace", opts.namespace);
	detail("Pool", opts.pool);
	if (opts.filter) detail("Filter", opts.filter);
	if (opts.region) detail("Region", opts.region);
	console.log(
		`\n${colors.dim("Streaming logs — press Ctrl+C to stop.")}\n`,
	);

	const controller = new AbortController();

	process.on("SIGINT", () => {
		controller.abort();
		console.log(`\n${colors.dim("Stopped.")}`);
		process.exit(0);
	});

	await streamWithRetry(client, project, org, opts, controller.signal);
}

async function streamWithRetry(
	client: CloudClient,
	project: string,
	org: string,
	opts: LogsOptions,
	signal: AbortSignal,
): Promise<void> {
	for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
		if (signal.aborted) return;

		try {
			const stream = client.streamLogs(project, opts.namespace, opts.pool, {
				contains: opts.filter,
				region: opts.region,
				signal,
			});

			for await (const entry of stream) {
				if (signal.aborted) return;
				printLogLine(entry);
			}

			// Stream ended cleanly
			return;
		} catch (err: unknown) {
			if (signal.aborted) return;
			if ((err as Error).name === "AbortError") return;

			if (attempt < MAX_RETRIES) {
				const delay = BASE_DELAY_MS * 2 ** attempt;
				console.error(
					colors.dim(
						`  Connection lost. Reconnecting in ${delay}ms… (attempt ${attempt + 1}/${MAX_RETRIES})`,
					),
				);
				await sleep(delay, signal);
			} else {
				fatal(
					`Failed to connect to log stream after ${MAX_RETRIES} retries.`,
					String(err),
				);
			}
		}
	}
}

function printLogLine(entry: {
	timestamp: string;
	region: string;
	message: string;
}): void {
	const ts = formatTimestamp(entry.timestamp);
	const region = formatRegion(entry.region);
	const msg = entry.message;
	console.log(`${ts} ${region} ${msg}`);
}

function sleep(ms: number, signal: AbortSignal): Promise<void> {
	return new Promise<void>((resolve) => {
		const timeout = setTimeout(resolve, ms);
		signal.addEventListener("abort", () => {
			clearTimeout(timeout);
			resolve();
		});
	});
}
