import { fork } from "node:child_process";
import {
	readFileSync,
	writeFileSync,
	openSync,
	closeSync,
	mkdirSync,
	mkdtempSync,
} from "node:fs";
import { resolve, dirname } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { createServer } from "node:net";
import { setImmediate as nextTick } from "node:timers/promises";
const script = fileURLToPath(import.meta.url);
const runner = process.argv[2] === "runner";
const { values } = parseArgs({
	args: runner ? [] : process.argv.slice(2),
	options: {
		output: { type: "string" },
		sessions: { type: "string", default: "100" },
	},
});
const count = Number(values.sessions);
if (!Number.isInteger(count) || count < 2 || count > 10000)
	throw new Error("--sessions must be an integer from 2 to 10000");
if (process.platform !== "linux")
	throw new Error(
		"This benchmark requires Linux /proc and POSIX process groups",
	);
let dir = process.env.PI_BENCH_OUTPUT;
if (!runner) {
	if (values.output) {
		dir = resolve(values.output);
		mkdirSync(dirname(dir), { recursive: true });
		mkdirSync(dir);
	} else dir = mkdtempSync(resolve(tmpdir(), "pi-session-memory-"));
	console.log(`Benchmark output: ${dir}`);
}
if (!dir) throw new Error("Runner requires PI_BENCH_OUTPUT");
if (process.argv[2] === "runner") {
	const { pi } = await import("../../dist/index.js");
	const { setup } = await import("rivetkit");
	const { ModelRuntime } = await import("@earendil-works/pi-coding-agent");
	const modelRuntime = await ModelRuntime.create({
		authPath: `${dir}/auth.json`,
		modelsPath: null,
	});
	modelRuntime.registerProvider("mock", {
		baseUrl: process.env.MOCK_LLM_URL + "/v1",
		api: "openai-completions",
		apiKey: "mock",
		models: [
			{
				id: "mock-model",
				name: "Mock model",
				reasoning: false,
				input: ["text"],
				cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
				contextWindow: 128000,
				maxTokens: 4096,
			},
		],
	});
	await modelRuntime.refresh({ allowNetwork: false });
	const mock = {
		model: modelRuntime.getModel("mock", "mock-model"),
		modelRuntime,
	};
	let live = 0;
	const agent = pi({
		model: mock.model,
		modelRuntime: mock.modelRuntime,
		options: { noSleep: true },
		createVars() {
			live++;
			return {};
		},
		onSleep() {
			live--;
		},
		onDestroy() {
			live--;
		},
	});
	const registry = setup({ use: { agent }, noWelcome: true });
	registry.start();
	for (let attempt = 0; attempt < 120; attempt++) {
		if ((await registry.routes.health()).ok) break;
		if (attempt === 119) throw new Error("Runner did not become ready");
		await new Promise((r) => setTimeout(r, 100));
	}
	process.on("message", async (msg) => {
		if (msg !== "measure") return;
		const beforeGc = process.memoryUsage();
		for (let i = 0; i < 3; i++) {
			global.gc();
			await nextTick();
		}
		const memory = process.memoryUsage();
		const smaps = readFileSync("/proc/self/smaps_rollup", "utf8");
		const privateKiB = [
			...smaps.matchAll(/^Private_(?:Clean|Dirty):\s+(\d+) kB/gm),
		].reduce((s, m) => s + Number(m[1]), 0);
		process.send({
			type: "measurement",
			pid: process.pid,
			live,
			beforeGc,
			...memory,
			privateBytes: privateKiB * 1024,
		});
	});
	process.send({ type: "ready" });
} else {
	const { LLMock } = await import("@copilotkit/aimock");
	const mock = new LLMock({ port: 0 });
	await mock.start();
	const server = createServer();
	await new Promise((r) => server.listen(0, "127.0.0.1", r));
	const port = server.address().port;
	await new Promise((r) => server.close(r));
	const log = openSync(`${dir}/runner.log`, "w");
	const child = fork(script, ["runner"], {
		execArgv: ["--expose-gc"],
		detached: true,
		env: {
			...process.env,
			PI_BENCH_OUTPUT: dir,
			MOCK_LLM_URL: mock.url,
			RIVET_RUN_ENGINE: "1",
			RIVET_RUN_ENGINE_PORT: String(port),
			RIVET_RUN_SERVICES: "0",
			RIVETKIT_STORAGE_PATH: `${dir}/storage`,
			RIVET_LOG_LEVEL: "ERROR",
			RUST_LOG: "error",
		},
		stdio: ["ignore", log, log, "ipc"],
	});
	closeSync(log);
	const signalRunner = (signal) => {
		try {
			process.kill(-child.pid, signal);
		} catch (error) {
			if (error.code !== "ESRCH") throw error;
		}
	};
	const interrupted = () => signalRunner("SIGTERM");
	process.once("SIGINT", interrupted);
	process.once("SIGTERM", interrupted);
	const nextMessage = () =>
		new Promise((resolve, reject) => {
			const cleanup = () => {
				clearTimeout(timer);
				child.off("message", message);
				child.off("error", error);
				child.off("exit", exit);
			};
			const message = (value) => {
				cleanup();
				resolve(value);
			};
			const error = (err) => {
				cleanup();
				reject(err);
			};
			const exit = (code, signal) =>
				error(
					new Error(`Runner exited (${code ?? signal}); see ${dir}/runner.log`),
				);
			const timer = setTimeout(
				() => error(new Error("Runner response timed out")),
				120000,
			);
			child.once("message", message);
			child.once("error", error);
			child.once("exit", exit);
			if (child.exitCode !== null || child.signalCode !== null)
				exit(child.exitCode, child.signalCode);
		});
	let client;
	try {
		const ready = await nextMessage();
		if (ready.type !== "ready")
			throw new Error("Unexpected runner readiness message");
		const { createClient } = await import("rivetkit/client");
		client = createClient({
			endpoint: `http://127.0.0.1:${port}`,
			namespace: "default",
		});
		const rows = [];
		const started = Date.now();
		async function sample(stage, live) {
			const result = nextMessage();
			child.send("measure");
			const row = await result;
			if (row.live !== live)
				throw new Error(`Expected ${live} live sessions, got ${row.live}`);
			rows.push({ stage, elapsedMs: Date.now() - started, ...row });
			writeFileSync(`${dir}/samples.json`, JSON.stringify(rows));
			console.log(JSON.stringify(rows.at(-1)));
		}
		await sample("empty", 0);
		const sessionIds = new Set();
		const verified = [];
		for (let n = 0; n < count; n++) {
			const handle = client.agent.getOrCreate([`prompt-memory-${n}`]);
			let session;
			for (let attempt = 0; attempt < 4; attempt++) {
				try {
					session = await handle.getSession();
					break;
				} catch (error) {
					if (error?.code !== "route_resolve_query_timeout" || attempt === 3)
						throw error;
					console.log(`Retrying creation of session ${n} after route timeout`);
				}
			}
			sessionIds.add(session.sessionId);
			const prompt = `Memory benchmark session ${String(n).padStart(3, "0")}: explain why session isolation matters.`;
			const expected = `Session ${n}: Each coding-agent session keeps its conversation and working state separate. This prevents one task from changing another task's context, lets users resume independent work, and makes failures easier to contain. Durable storage preserves the conversation when an actor sleeps. Shared model configuration avoids duplicating static setup while each session retains its own messages. This is a deterministic mock response used to measure runner memory after a complete prompt and response cycle.`;
			mock.onMessage(
				prompt,
				{ content: expected },
				{ streamingProfile: { ttft: 0, tps: 100000 } },
			);
			await handle.prompt(prompt);
			await handle.waitForIdle();
			const messages = await handle.getMessages();
			const assistant = messages.filter((m) => m.role === "assistant").at(-1);
			const text = assistant?.content
				.filter((c) => c.type === "text")
				.map((c) => c.text)
				.join("");
			if (text !== expected || assistant.stopReason === "error")
				throw new Error(
					`Session ${n} response did not match: ${JSON.stringify(assistant)}`,
				);
			if (
				!messages.some(
					(m) =>
						m.role === "user" && JSON.stringify(m.content).includes(prompt),
				)
			)
				throw new Error(`Session ${n} missing user prompt`);
			verified.push({
				session: n,
				messageCount: messages.length,
				responseChars: text.length,
			});
			await sample(`${n + 1} responded`, n + 1);
		}
		if (sessionIds.size !== count)
			throw new Error(`Did not create ${count} distinct Pi sessions`);
		const requests = mock.getRequests();
		if (requests.length !== count)
			throw new Error(
				`Expected ${count} LLM HTTP requests, got ${requests.length}`,
			);
		writeFileSync(
			`${dir}/results.json`,
			JSON.stringify(
				{
					node: process.version,
					sessionCount: count,
					runnerOnly: true,
					sandbox: false,
					prompted: true,
					concurrency: 1,
					llmRequests: requests.length,
					verifiedResponses: verified.length,
					uniquePiSessions: sessionIds.size,
					rows,
					verified,
				},
				null,
				2,
			),
		);
		console.log(
			`Verified ${verified.length} responses, ${requests.length} mock LLM HTTP requests, ${sessionIds.size} distinct Pi sessions.`,
		);
	} finally {
		try {
			if (client) await client.dispose();
		} finally {
			signalRunner("SIGTERM");
			await new Promise((resolve) => setTimeout(resolve, 1000));
			signalRunner("SIGKILL");
			process.off("SIGINT", interrupted);
			process.off("SIGTERM", interrupted);
			await mock.stop();
		}
	}
}
