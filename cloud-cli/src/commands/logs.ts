/**
 * `rivet-cloud logs` — stream real-time logs from a Rivet Cloud managed pool.
 *
 * The implementation mirrors the frontend's use-deployment-logs-stream.ts:
 * it opens an SSE connection to the Cloud API log endpoint via RivetSse from
 * @rivet-gg/cloud, reconnects on failure with exponential back-off (up to 8
 * retries), and prints every log line to stdout.
 */

import type { Command } from "commander";
import { RivetSse, type Rivet } from "@rivet-gg/cloud";
import { createCloudClient } from "../lib/client.ts";
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
	const client = createCloudClient({ token, baseUrl: opts.apiUrl });
	const apiUrl = opts.apiUrl ?? "https://cloud-api.rivet.dev";

	// Resolve org + project from token
	let identity: { project: string; organization: string };

	try {
		identity = await client.apiTokens.inspect();
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

	await streamWithRetry(token, apiUrl, project, org, opts, controller.signal);
}

async function streamWithRetry(
	token: string,
	apiUrl: string,
	project: string,
	org: string,
	opts: LogsOptions,
	signal: AbortSignal,
): Promise<void> {
	const streamOptions = {
		environment: "",
		baseUrl: apiUrl,
		token,
	};

	for (let attempt = 0; attempt <= MAX_RETRIES; attempt++) {
		if (signal.aborted) return;

		try {
			const stream = RivetSse.streamLogs(
				streamOptions,
				project,
				opts.namespace,
				opts.pool,
				{
					contains: opts.filter,
					region: opts.region,
					abortSignal: signal,
				},
			);

			for await (const event of stream) {
				if (signal.aborted) return;

				if (event.event === "connected") {
					// Connection established — no action needed.
				} else if (event.event === "end") {
					return;
				} else if (event.event === "error") {
					throw new Error(event.data.message);
				} else if (event.event === "log") {
					printLogLine(event.data);
				}
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

function printLogLine(data: Rivet.LogStreamEvent.Log["data"]): void {
	const ts = formatTimestamp(data.timestamp);
	const region = formatRegion(data.region);
	const msg = data.message;
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

