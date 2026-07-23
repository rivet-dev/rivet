// @ts-nocheck
import assert from 'node:assert/strict';
import { execFileSync, spawn, spawnSync } from 'node:child_process';
import { once } from 'node:events';
import * as fs from 'node:fs';
import { createRequire } from 'node:module';
import { createServer as createNetServer } from 'node:net';
import * as os from 'node:os';
import * as path from 'node:path';
import { test } from 'node:test';

const packageRoot = path.resolve(import.meta.dirname, '..');
const rivetExampleRoot = path.join(packageRoot, 'examples', 'rivet');
const vercelExampleRoot = path.join(packageRoot, 'examples', 'rivet-vercel');
const require = createRequire(import.meta.url);
const { getEnginePath } = require('@rivetkit/engine-cli');

test('examples/rivet builds and serves agent and workflow requests end to end', { timeout: 180_000 }, async () => {
	const port = await getAvailablePort();
	const enginePort = await getAvailablePort();
	const instanceId = `example-${Date.now()}-${Math.random().toString(36).slice(2)}`;
	const poolName = `flue-example-${Date.now()}-${Math.random().toString(36).slice(2)}`;
	const registryKey = `registry-${Date.now()}-${Math.random().toString(36).slice(2)}`;

	const build = await runCommand('pnpm', ['run', 'build'], rivetExampleRoot);
	assert.equal(build.code, 0, build.stderr);
	assert.equal(fs.existsSync(path.join(rivetExampleRoot, 'dist', 'server.mjs')), true);

	const syntax = spawnSync(process.execPath, ['--check', path.join(rivetExampleRoot, 'dist', 'server.mjs')], {
		encoding: 'utf8',
	});
	assert.equal(syntax.status, 0, syntax.stderr);

	const enginePeerPort = await getAvailablePort();
	const engineMetricsPort = await getAvailablePort();
	const engine = startEngine({
		guardPort: enginePort,
		peerPort: enginePeerPort,
		metricsPort: engineMetricsPort,
	});
	let dev;
	try {
		await waitForMetadata(enginePort, engine.logs);
		await configureNormalRunnerConfig(poolName, enginePort);
		dev = startDev(rivetExampleRoot, port, {
			RIVET_RUN_ENGINE: '0',
			RIVET_ENDPOINT: `http://127.0.0.1:${enginePort}`,
			RIVET_POOL: poolName,
			FLUE_RIVET_REGISTRY_KEY: registryKey,
		});
		await waitForServer(port, dev.logs);

		const admitted = await postAgentPromptWithRetry(
			(attempt) => `http://127.0.0.1:${port}/agents/assistant/${instanceId}-${attempt}`,
			'Hello example',
			() => `${dev.logs()}\n\n${engine.logs()}`,
		);
		const promptInstanceId = `${instanceId}-${admitted.attempt}`;
		assert.equal(typeof admitted.receipt.submissionId, 'string');
		const conversation = await waitForConversationText(
			`http://127.0.0.1:${port}/agents/assistant/${promptInstanceId}?view=history`,
			'Hello from Rivet.',
			dev.logs,
		);

		// Regression coverage for the streaming forwarding rework (#1): an SSE read must
		// stream `text/event-stream` immediately. Under the old serialize-to-JSON shim
		// the actor buffered the never-ending stream until the action timeout, so this
		// fetch would hang instead of resolving with headers.
		const sse = await fetchWithTimeout(
			`http://127.0.0.1:${port}/agents/assistant/${promptInstanceId}?view=updates&live=sse&offset=${encodeURIComponent(conversation.offset)}`,
			undefined,
			30_000,
		);
		assert.equal(sse.status, 200, dev.logs());
		assert.match(sse.headers.get('content-type') ?? '', /text\/event-stream/);
		await sse.body?.cancel();

		const workflow = await fetchWithTimeout(
			`http://127.0.0.1:${port}/workflows/dispatch?wait=result`,
			{
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({
					source: 'examples/rivet',
					dispatchedInstanceId: `dispatched-${instanceId}`,
				}),
			},
			30_000,
		);
		const workflowText = await workflow.text();
		assert.equal(workflow.status, 200, `${workflowText}\n\n${dev.logs()}`);
		const workflowBody = JSON.parse(workflowText);
		assert.equal(workflowBody.result.input.source, 'examples/rivet');
		assert.equal(workflowBody.result.input.dispatchedInstanceId, `dispatched-${instanceId}`);
		assert.equal(typeof workflowBody.result.dispatchId, 'string');

		// Prove the dispatch was actually DELIVERED, not just enqueued: the workflow
		// dispatched to agent `assistant` instance `dispatched-${instanceId}`, whose
		// faux provider answers the dispatched turn with "Hello from workflow
		// dispatch." That output lands in the target instance's own event stream, so
		// poll its catch-up stream until the assistant text appears.
		await waitForAgentStreamText(
			`http://127.0.0.1:${port}/agents/assistant/dispatched-${instanceId}`,
			'Hello from workflow dispatch.',
			dev.logs,
		);

		await fetchOkTextWithRetry(
			`http://127.0.0.1:${port}/runs/${workflowBody.runId}?meta`,
			undefined,
			dev.logs,
		);

		const runListText = await fetchOkTextWithRetry(
			`http://127.0.0.1:${port}/admin/runs`,
			undefined,
			dev.logs,
		);
		assert.ok(
			JSON.parse(runListText).runs.some((run) => run.runId === workflowBody.runId),
			runListText,
		);

		const runStreamText = await fetchOkTextWithRetry(
			`http://127.0.0.1:${port}/runs/${workflowBody.runId}`,
			undefined,
			dev.logs,
		);
		assert.match(runStreamText, /run_start|run_end/);

		// Regression coverage for null-body forwarding (#2): a completed run's event
		// stream is closed, so a long-poll returns a null-body 204. The old serialize
		// shim crashed reconstructing `new Response(<buffer>, { status: 204 })`; raw
		// streaming passes the null body through untouched.
		const runLongPoll = await fetchWithTimeout(
			`http://127.0.0.1:${port}/runs/${workflowBody.runId}?live=long-poll&offset=now`,
			undefined,
			30_000,
		);
		assert.equal(runLongPoll.status, 204, dev.logs());
	} finally {
		await dev?.stop();
		await engine.stop();
		killEngineOnPort(enginePort);
	}
});

test('examples/rivet-vercel builds and exposes Next route handlers end to end', { timeout: 180_000 }, async () => {
	const build = await runCommand('pnpm', ['run', 'flue:build'], vercelExampleRoot);
	assert.equal(build.code, 0, build.stderr);
	assert.equal(fs.existsSync(path.join(vercelExampleRoot, 'dist', 'server.mjs')), true);

	const previousEnv = {
		RIVET_RUN_ENGINE: process.env.RIVET_RUN_ENGINE,
		RIVET_RUN_ENGINE_HOST: process.env.RIVET_RUN_ENGINE_HOST,
		RIVET_RUN_ENGINE_PORT: process.env.RIVET_RUN_ENGINE_PORT,
		RIVET_ENDPOINT: process.env.RIVET_ENDPOINT,
		FLUE_RIVET_ENDPOINT: process.env.FLUE_RIVET_ENDPOINT,
		NEXT_PUBLIC_SITE_URL: process.env.NEXT_PUBLIC_SITE_URL,
		RIVET_POOL: process.env.RIVET_POOL,
		FLUE_RIVET_REGISTRY_KEY: process.env.FLUE_RIVET_REGISTRY_KEY,
		RIVETKIT_RUNTIME_MODE: process.env.RIVETKIT_RUNTIME_MODE,
		RIVET_PUBLIC_ENDPOINT: process.env.RIVET_PUBLIC_ENDPOINT,
		RIVET_PUBLIC_TOKEN: process.env.RIVET_PUBLIC_TOKEN,
	};
	const poolName = `flue-vercel-${Date.now()}-${Math.random().toString(36).slice(2)}`;
	const registryKey = `registry-${Date.now()}-${Math.random().toString(36).slice(2)}`;
	const enginePort = await getAvailablePort();
	const enginePeerPort = await getAvailablePort();
	const engineMetricsPort = await getAvailablePort();
	process.env.RIVET_RUN_ENGINE = '1';
	process.env.RIVET_RUN_ENGINE_HOST = '127.0.0.1';
	process.env.RIVET_RUN_ENGINE_PORT = String(enginePort);
	process.env.RIVET_POOL = poolName;
	process.env.FLUE_RIVET_REGISTRY_KEY = registryKey;
	delete process.env.RIVET_ENDPOINT;
	delete process.env.FLUE_RIVET_ENDPOINT;
	delete process.env.NEXT_PUBLIC_SITE_URL;
	delete process.env.RIVETKIT_RUNTIME_MODE;
	delete process.env.RIVET_PUBLIC_ENDPOINT;
	delete process.env.RIVET_PUBLIC_TOKEN;

	const port = await getAvailablePort();
	const origin = `http://127.0.0.1:${port}`;
	let engine;
	let dev;
	let passed = false;
	try {
		engine = startEngine({
			guardPort: enginePort,
			peerPort: enginePeerPort,
			metricsPort: engineMetricsPort,
		});
		await waitForMetadata(enginePort, engine.logs);

		dev = startNextDev(vercelExampleRoot, port, {
			RIVET_RUN_ENGINE: '1',
			RIVET_RUN_ENGINE_HOST: '127.0.0.1',
			RIVET_RUN_ENGINE_PORT: String(enginePort),
			RIVET_POOL: poolName,
			FLUE_RIVET_REGISTRY_KEY: registryKey,
			FLUE_RIVET_ENDPOINT: `${origin}/api/rivet`,
			NEXT_PUBLIC_SITE_URL: origin,
		});
		await waitForHttpOk(`${origin}/api/rivet/metadata`, dev.logs, 60_000);

		const instanceId = `vercel-example-${Date.now()}-${Math.random().toString(36).slice(2)}`;
		const admitted = await postAgentPromptWithRetry(
			(attempt) => `${origin}/api/flue/agents/assistant/${instanceId}-${attempt}`,
			'Hello from example route',
			dev.logs,
		);
		assert.equal(typeof admitted.receipt.submissionId, 'string');
		await waitForConversationText(
			`${origin}/api/flue/agents/assistant/${instanceId}-${admitted.attempt}?view=history`,
			'Hello from the Vercel route.',
			dev.logs,
		);
		await waitForLog(dev.logs, /POST \/api\/rivet\/start/, 10_000);

		const gateway = await fetchWithTimeout(`${origin}/api/rivet/metadata`, undefined, 30_000);
		assert.equal(gateway.status, 200, await gateway.text());
		passed = true;
	} finally {
		await delay(250);
		await dev?.stop();
		await engine?.stop();
		restoreEnv(previousEnv);
		setTimeout(() => process.exit(passed ? 0 : 1), 100);
	}
});

async function postAgentPromptWithRetry(urlForAttempt, body, logs = () => '') {
	let lastText = '';
	for (let attempt = 0; attempt < 6; attempt++) {
		try {
			const response = await fetchWithTimeout(urlForAttempt(attempt), {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ kind: 'user', body }),
			}, 30_000);
			lastText = await response.text();
			if (response.status === 202) return { attempt, receipt: JSON.parse(lastText) };
			if (response.status < 500 && !/actor|envoy|ready|wake|timeout|aborted/i.test(lastText)) {
				assert.equal(response.status, 202, `${lastText}\n\n${logs()}`);
			}
		} catch (error) {
			lastText = error instanceof Error ? error.stack || error.message : String(error);
			if (!/actor|envoy|ready|wake|timeout|aborted|fetch failed/i.test(lastText)) throw error;
		}
		await delay(2_000 * (attempt + 1));
	}
	assert.fail(`${lastText}\n\n${logs()}`);
}

async function waitForConversationText(url, expectedText, logs = () => '') {
	let lastText = '';
	for (let attempt = 0; attempt < 20; attempt++) {
		if (attempt > 0) await delay(250);
		const response = await fetchWithTimeout(url, undefined, 30_000);
		lastText = await response.text();
		if (!response.ok) continue;
		const snapshot = JSON.parse(lastText);
		if (snapshot.messages?.some((message) =>
			message.role === 'assistant' &&
			message.parts?.some((part) => part.type === 'text' && part.text === expectedText),
		)) return snapshot;
	}
	assert.fail(`Expected conversation to contain ${JSON.stringify(expectedText)}\n\n${lastText}\n\n${logs()}`);
}

async function waitForAgentStreamText(url, expectedText, logs = () => '') {
	let lastText = '';
	for (let attempt = 0; attempt < 6; attempt++) {
		if (attempt > 0) await delay(1_500 * attempt);
		// Dispatch is asynchronous, so the target instance's stream may not exist
		// yet (404) until its first event is persisted; tolerate that and retry.
		const response = await fetchWithTimeout(url, undefined, 30_000);
		lastText = await response.text();
		if (response.status === 200 && lastText.includes(expectedText)) return lastText;
	}
	assert.fail(
		`Expected dispatched agent stream to contain ${JSON.stringify(expectedText)}\n\n${lastText}\n\n${logs()}`,
	);
}

async function fetchOkTextWithRetry(url, init, logs = () => '') {
	let lastText = '';
	for (let attempt = 0; attempt < 4; attempt++) {
		const response = await fetchWithTimeout(url, init, 30_000);
		lastText = await response.text();
		if (response.status === 200) return lastText;
		const diagnostics = `${lastText}\n\n${logs()}`;
		if (!/actor|envoy|ready|wake/i.test(diagnostics)) {
			assert.equal(response.status, 200, diagnostics);
		}
		await delay(2_000 * (attempt + 1));
	}
	assert.fail(`${lastText}\n\n${logs()}`);
}

async function runCommand(command, args, cwd) {
	const child = spawn(command, args, {
		cwd,
		stdio: ['ignore', 'pipe', 'pipe'],
	});
	let stdout = '';
	let stderr = '';
	child.stdout.setEncoding('utf8');
	child.stderr.setEncoding('utf8');
	child.stdout.on('data', (chunk) => {
		stdout += chunk;
	});
	child.stderr.on('data', (chunk) => {
		stderr += chunk;
	});
	const [code, signal] = await once(child, 'exit');
	return { code, signal, stdout, stderr };
}

function startDev(cwd, port, env = {}) {
	const child = spawn('pnpm', ['exec', 'flue', 'dev', '--port', String(port)], {
		cwd,
		env: {
			...process.env,
			...env,
			RIVET_RUN_ENGINE: env.RIVET_RUN_ENGINE ?? '1',
		},
		stdio: ['ignore', 'pipe', 'pipe'],
	});
	let output = '';
	for (const stream of [child.stdout, child.stderr]) {
		stream.setEncoding('utf8');
		stream.on('data', (chunk) => {
			output += chunk;
		});
	}
	return {
		logs() {
			return output;
		},
		async stop() {
			if (child.exitCode !== null || child.signalCode !== null) return;
			child.kill('SIGTERM');
			await Promise.race([
				once(child, 'exit'),
				new Promise((_, reject) =>
					setTimeout(() => reject(new Error(`Timed out stopping flue dev\n\n${output}`)), 5_000),
				),
			]);
		},
	};
}

function startNextDev(cwd, port, env) {
	const childEnv = {
		...process.env,
		...env,
		RIVET_RUN_ENGINE_HOST: env.RIVET_RUN_ENGINE_HOST || '127.0.0.1',
		RIVET_RUN_ENGINE_PORT: env.RIVET_RUN_ENGINE_PORT,
	};
	delete childEnv.RIVET_ENDPOINT;
	delete childEnv.RIVETKIT_RUNTIME_MODE;
	delete childEnv.RIVET_PUBLIC_ENDPOINT;
	delete childEnv.RIVET_PUBLIC_TOKEN;
	const child = spawn('pnpm', ['exec', 'next', 'dev', '--webpack', '--port', String(port)], {
		cwd,
		env: childEnv,
		stdio: ['ignore', 'pipe', 'pipe'],
	});
	let output = '';
	for (const stream of [child.stdout, child.stderr]) {
		stream.setEncoding('utf8');
		stream.on('data', (chunk) => {
			output += chunk;
		});
	}
	return {
		logs() {
			return output;
		},
		async stop() {
			if (child.exitCode !== null || child.signalCode !== null) return;
			child.kill('SIGINT');
			await Promise.race([
				once(child, 'exit'),
				new Promise((_, reject) =>
					setTimeout(() => reject(new Error(`Timed out stopping next dev\n\n${output}`)), 5_000),
				),
			]);
		},
	};
}

function startEngine({ guardPort, peerPort, metricsPort }) {
	const root = fs.mkdtempSync(path.join(os.tmpdir(), 'flue-rivet-engine-'));
	const configPath = path.join(root, 'config.json');
	fs.writeFileSync(
		configPath,
		JSON.stringify({
			api_peer: { port: peerPort },
			file_system: { path: path.join(root, 'db') },
			guard: { port: guardPort },
			metrics: { port: metricsPort },
			telemetry: { enabled: false },
			topology: {
				datacenter_label: 1,
				datacenters: {
					default: {
						datacenter_label: 1,
						is_leader: true,
						peer_url: `http://127.0.0.1:${peerPort}`,
						public_url: `http://127.0.0.1:${guardPort}`,
					},
				},
			},
		}),
	);
	const child = spawn(getEnginePath(), ['start', '--config', configPath], {
		stdio: ['ignore', 'pipe', 'pipe'],
	});
	let output = '';
	for (const stream of [child.stdout, child.stderr]) {
		stream.setEncoding('utf8');
		stream.on('data', (chunk) => {
			output += chunk;
		});
	}
	return {
		logs() {
			return output;
		},
		async stop() {
			if (child.exitCode === null && child.signalCode === null) {
				child.kill('SIGTERM');
				await Promise.race([
					once(child, 'exit'),
					new Promise((_, reject) =>
						setTimeout(() => reject(new Error(`Timed out stopping rivet-engine\n\n${output}`)), 5_000),
					),
				]).catch(() => child.kill('SIGKILL'));
			}
			fs.rmSync(root, { recursive: true, force: true });
		},
	};
}

async function getAvailablePort() {
	const server = createNetServer();
	server.listen(0, '127.0.0.1');
	await once(server, 'listening');
	const address = server.address();
	assert(address && typeof address === 'object');
	server.close();
	await once(server, 'close');
	return address.port;
}

async function waitForHttpOk(url, logs = () => '', timeout = 20_000) {
	await waitFor(
		async () => {
			try {
				const response = await fetch(url);
				return response.ok;
			} catch {
				return false;
			}
		},
		() => `Timed out waiting for ${url}\n\n${logs()}`,
		timeout,
	);
}

async function waitForLog(logs, pattern, timeout = 20_000) {
	await waitFor(
		() => pattern.test(logs()),
		() => `Timed out waiting for log pattern ${pattern}\n\n${logs()}`,
		timeout,
	);
}

async function waitForServer(port, logs = () => '') {
	await waitFor(
		async () => {
			try {
				const response = await fetch(`http://127.0.0.1:${port}/`);
				return response.status < 500;
			} catch {
				return false;
			}
		},
		() => `Timed out waiting for server on port ${port}\n\n${logs()}`,
	);
}

async function waitForMetadata(port, logs = () => '', timeout = 45_000) {
	const deadline = Date.now() + timeout;
	while (Date.now() < deadline) {
		try {
			const response = await fetch(`http://127.0.0.1:${port}/metadata`);
			if (response.ok) return;
		} catch {}
		await delay(250);
	}
	throw new Error(`Timed out waiting for Rivet metadata on port ${port}\n\n${logs()}`);
}

async function configureNormalRunnerConfig(poolName, port) {
	const datacenters = await fetch(`http://127.0.0.1:${port}/datacenters`);
	const datacentersText = await datacenters.text();
	assert.equal(datacenters.status, 200, datacentersText);
	const body = JSON.parse(datacentersText);
	const response = await fetch(`http://127.0.0.1:${port}/runner-configs/${poolName}?namespace=default`, {
		method: 'PUT',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({
			datacenters: Object.fromEntries(
				body.datacenters.map((dc) => [
					dc.name,
					{
						normal: {
							drain_on_version_upgrade: false,
						},
						metadata: {},
					},
				]),
			),
		}),
	});
	assert.equal(response.status, 200, await response.text());
}

async function waitFor(predicate, message, timeout = 20_000) {
	const deadline = Date.now() + timeout;
	while (Date.now() < deadline) {
		if (await predicate()) return;
		await delay(50);
	}
	throw new Error(typeof message === 'function' ? message() : message);
}

function delay(ms) {
	return new Promise((resolve) => setTimeout(resolve, ms));
}

async function fetchWithTimeout(url, init, timeout) {
	try {
		return await fetch(url, { ...init, signal: AbortSignal.timeout(timeout) });
	} catch (error) {
		throw new Error(`Request failed for ${url}`, { cause: error });
	}
}

function restoreEnv(previousEnv) {
	for (const [key, value] of Object.entries(previousEnv)) {
		if (value === undefined) {
			delete process.env[key];
		} else {
			process.env[key] = value;
		}
	}
}

function killEngineOnPort(port) {
	try {
		const output = execFileSync('ss', ['-ltnp'], { encoding: 'utf8' });
		const portPattern = new RegExp(`:${port}\\b`);
		for (const line of output.split('\n')) {
			if (!portPattern.test(line)) continue;
			const match = line.match(/pid=(\d+)/);
			if (match) process.kill(Number(match[1]), 'SIGKILL');
		}
	} catch {}
}
