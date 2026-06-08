/**
 * Driver for slowReconnectActor.
 *
 * Warm baseline:
 *   pnpm slow-reconnect -- --warm --endpoint http://localhost:6420
 *
 * Default cold wake loop:
 *   pnpm slow-reconnect
 */

import { createClient } from "rivetkit/client";
import type { registry } from "./slow-reconnect-actor";

interface SlowReconnectStep {
	name: string;
	durationMs: number;
	rowCount: number;
}

interface SlowReconnectWorkloadResult {
	name: string;
	totalMs: number;
	steps: SlowReconnectStep[];
}

interface SlowReconnectResultMessage {
	type: "slow_reconnect_result";
	trigger: string;
	totalMs: number;
	results: SlowReconnectWorkloadResult[];
}

interface SlowReconnectErrorMessage {
	type: "slow_reconnect_error";
	trigger: string;
	error: string;
}

type ActorMessage =
	| SlowReconnectResultMessage
	| SlowReconnectErrorMessage
	| { type: string };

const args = process.argv.slice(2);
const endpoint =
	readFlagValue("--endpoint") ??
	process.env.RIVET_PUBLIC_ENDPOINT ??
	process.env.RIVET_ENDPOINT ??
	"http://localhost:6420";
const poolName = readFlagValue("--pool") ?? process.env.RIVET_POOL ?? "k8s";
const key =
	readFlagValue("--key") ??
	`slow-reconnect-${timestampSlug()}-${randomSuffix()}`;
const runs = Number(readFlagValue("--runs") ?? "3");
const wakeLoops = Number(
	readFlagValue("--wake-loops") ?? readFlagValue("--sleep-loops") ?? "5",
);
const timeoutMs = Number(readFlagValue("--timeout-ms") ?? "120000");
const mode = readFlagValue("--mode") ?? "executor_connect";
const staggerHandleMs = Number(readFlagValue("--stagger-handle-ms") ?? "0");
const loop = args.includes("--loop");
const cold = !(args.includes("--warm") || args.includes("--no-cold"));
const prepare = !args.includes("--no-prepare");
const sleepMs = Number(readFlagValue("--sleep-ms") ?? "1000");
const reconnectDelayMs = Number(
	readFlagValue("--reconnect-delay-ms") ?? "1000",
);

if (
	mode !== "executor_connect" &&
	mode !== "repro_reconnect" &&
	mode !== "client_resume"
) {
	console.error(
		"Usage: --mode must be executor_connect, repro_reconnect, or client_resume",
	);
	process.exit(1);
}
if (!Number.isInteger(runs) || runs < 1) {
	console.error("Usage: --runs must be an integer >= 1");
	process.exit(1);
}
if (!Number.isInteger(wakeLoops) || wakeLoops < 1) {
	console.error("Usage: --wake-loops/--sleep-loops must be an integer >= 1");
	process.exit(1);
}

console.log(
	`[slow-reconnect] endpoint=${endpoint} pool=${poolName ?? "<default>"} key=${key}`,
);
console.log(
	`[slow-reconnect] runs=${runs} wakeLoops=${loop ? "∞" : wakeLoops} timeout=${ms(timeoutMs)} mode=${mode} staggerHandleMs=${staggerHandleMs}`,
);
console.log(
	`[slow-reconnect] cold=${cold} prepare=${prepare} sleepMs=${sleepMs} reconnectDelayMs=${reconnectDelayMs}`,
);

const client = createClient<typeof registry>({
	endpoint,
	...(poolName ? { poolName } : {}),
});
let stopping = false;

process.on("SIGINT", () => {
	console.log("\n[slow-reconnect] SIGINT, stopping...");
	stopping = true;
});

try {
	if (prepare) {
		await prepareActor();
		if (cold) {
			await sleepActor(1);
		}
	}

	let globalRun = 1;
	let wakeLoop = 1;
	while (!stopping && (loop || wakeLoop <= wakeLoops)) {
		console.log(`\n[wake ${wakeLoop}] starting ${runs} reconnect run(s)`);
		for (
			let reconnectRun = 1;
			!stopping && reconnectRun <= runs;
			reconnectRun++
		) {
			try {
				const result = await runOnce(globalRun, wakeLoop, reconnectRun);
				printResult(globalRun, wakeLoop, reconnectRun, result);
			} catch (error) {
				console.error(
					`[wake ${wakeLoop} run ${reconnectRun}] failed:`,
					error,
				);
				if (!loop) {
					throw error;
				}
			}

			globalRun++;
			if (reconnectRun < runs && !stopping && reconnectDelayMs > 0) {
				await delay(reconnectDelayMs);
			}
		}

		wakeLoop++;
		if (cold && !stopping && (loop || wakeLoop <= wakeLoops)) {
			await sleepActor(wakeLoop);
		}
	}
} finally {
	await client.dispose();
}

async function prepareActor(): Promise<void> {
	const handle = client.slowReconnectActor.getOrCreate([key]);
	const startedAt = performance.now();
	console.log("\n[prepare] seeding slow reconnect actor...");
	const result = await (
		handle as unknown as {
			prepare: () => Promise<{
				seeded: boolean;
				messages: number;
				toolCalls: number;
				threadEvents: number;
			}>;
		}
	).prepare();
	console.log(
		`[prepare] seeded=${result.seeded} messages=${result.messages} toolCalls=${result.toolCalls} threadEvents=${result.threadEvents} in ${ms(performance.now() - startedAt)}`,
	);
}

async function sleepActor(nextWakeLoop: number): Promise<void> {
	const handle = client.slowReconnectActor.getOrCreate([key]);
	const startedAt = performance.now();
	console.log(`\n[wake ${nextWakeLoop}] sleeping actor...`);
	await (handle as unknown as { sleep: () => Promise<unknown> }).sleep();
	console.log(
		`[wake ${nextWakeLoop}] sleep action returned in ${ms(performance.now() - startedAt)}; waiting ${ms(sleepMs)} before reconnect`,
	);
	await delay(sleepMs);
}

async function runOnce(
	index: number,
	wakeLoop: number,
	reconnectRun: number,
): Promise<SlowReconnectResultMessage> {
	const handle = client.slowReconnectActor.getOrCreate([key]);
	const startedAt = performance.now();
	console.log(`[wake ${wakeLoop} run ${reconnectRun}] opening websocket...`);
	const ws = await handle.webSocket("/", undefined, { skipReadyWait: true });
	if (!ws) {
		throw new Error("slowReconnectActor did not return a WebSocket");
	}
	try {
		await waitForOpen(ws);
		console.log(
			`[wake ${wakeLoop} run ${reconnectRun}] websocket open in ${ms(performance.now() - startedAt)}`,
		);
		const resultPromise = waitForResult(ws, timeoutMs);
		ws.send(JSON.stringify(buildRequest(index)));
		return await resultPromise;
	} finally {
		ws.close();
	}
}

function buildRequest(index: number): object {
	if (mode === "client_resume") {
		return { type: "client_resume", version: 0 };
	}
	if (mode === "repro_reconnect") {
		return {
			type: "repro_reconnect",
			clientId: `slow-reconnect-client-${index}`,
			staggerHandleMs,
		};
	}
	return {
		type: "executor_connect",
		clientId: `slow-reconnect-client-${index}`,
		executorType: "local-client",
		capabilities: {},
	};
}

function waitForResult(
	ws: WebSocket,
	timeoutMs: number,
): Promise<SlowReconnectResultMessage> {
	return new Promise((resolve, reject) => {
		const timeout = setTimeout(() => {
			reject(
				new Error(
					`Timed out after ${timeoutMs}ms waiting for slowReconnectActor result`,
				),
			);
			try {
				ws.close();
			} catch {}
		}, timeoutMs);

		const cleanup = () => clearTimeout(timeout);

		ws.addEventListener("message", (event: MessageEvent) => {
			const data = typeof event.data === "string" ? event.data : "";
			if (data === "pong") {
				return;
			}
			let message: ActorMessage;
			try {
				message = JSON.parse(data) as ActorMessage;
			} catch {
				console.log(`[slow-reconnect] <<< ${data.slice(0, 200)}`);
				return;
			}
			if (message.type === "executor_connected") {
				console.log("[slow-reconnect] <<< executor_connected");
				return;
			}
			if (message.type === "slow_reconnect_error") {
				cleanup();
				reject(new Error((message as SlowReconnectErrorMessage).error));
				return;
			}
			if (message.type === "slow_reconnect_result") {
				cleanup();
				resolve(message as SlowReconnectResultMessage);
			}
		});

		ws.addEventListener("close", (event: CloseEvent) => {
			cleanup();
			reject(
				new Error(
					`WebSocket closed before result: code=${event.code} reason=${event.reason || "<empty>"}`,
				),
			);
		});
		ws.addEventListener("error", () => {
			cleanup();
			reject(new Error("WebSocket failed while waiting for result"));
		});
	});
}

async function waitForOpen(ws: WebSocket): Promise<void> {
	if (ws.readyState === WebSocket.OPEN) {
		return;
	}
	await new Promise<void>((resolve, reject) => {
		ws.addEventListener("open", () => resolve(), { once: true });
		ws.addEventListener(
			"error",
			() => reject(new Error("WebSocket failed to open")),
			{
				once: true,
			},
		);
		ws.addEventListener(
			"close",
			() => reject(new Error("WebSocket closed before open")),
			{
				once: true,
			},
		);
	});
}

function printResult(
	index: number,
	wakeLoop: number,
	reconnectRun: number,
	message: SlowReconnectResultMessage,
): void {
	console.log(
		`\n[wake ${wakeLoop} run ${reconnectRun} global ${index}] trigger=${message.trigger} total=${ms(message.totalMs)}`,
	);
	for (const workload of message.results) {
		console.log(
			`  ${workload.name.padEnd(28)} total=${ms(workload.totalMs)}`,
		);
		for (const step of workload.steps) {
			console.log(
				`    ${step.name.padEnd(36)} ${ms(step.durationMs).padStart(8)} rows=${step.rowCount}`,
			);
		}
	}
}

function readFlagValue(flag: string): string | undefined {
	const prefix = `${flag}=`;
	const equalsValue = args.find((arg) => arg.startsWith(prefix));
	if (equalsValue) {
		return equalsValue.slice(prefix.length);
	}
	const index = args.indexOf(flag);
	if (index === -1) {
		return undefined;
	}
	return args[index + 1];
}

function ms(value: number): string {
	return `${Math.round(value)}ms`;
}

function delay(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, Math.max(0, ms)));
}

function timestampSlug(): string {
	const d = new Date();
	const pad = (n: number) => String(n).padStart(2, "0");
	return `${d.getUTCFullYear()}${pad(d.getUTCMonth() + 1)}${pad(d.getUTCDate())}T${pad(d.getUTCHours())}${pad(d.getUTCMinutes())}${pad(d.getUTCSeconds())}`;
}

function randomSuffix(): string {
	return Math.random().toString(36).slice(2, 8);
}
