import assert from 'node:assert/strict';
import { execFileSync, spawn } from 'node:child_process';
import { once } from 'node:events';
import * as fs from 'node:fs';
import { createRequire } from 'node:module';
import { createServer } from 'node:net';
import * as os from 'node:os';
import * as path from 'node:path';
import { test } from 'node:test';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { toFlueNextHandler } from '../src/next.ts';

const packageRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const fixtureRoots = [];
const require = createRequire(import.meta.url);
const flueCliRoot = path.resolve(
	path.dirname(fileURLToPath(import.meta.resolve('@flue/cli'))),
	'..',
);
const flueRuntimeRoot = path.resolve(
	path.dirname(fileURLToPath(import.meta.resolve('@flue/runtime'))),
	'..',
);
const cli = pathToFileURL(path.join(flueCliRoot, 'bin', 'flue.mjs'));
const { getEnginePath } = require('@rivetkit/engine-cli');

process.on('exit', () => {
	for (const root of fixtureRoots) fs.rmSync(root, { recursive: true, force: true });
});

test('serves a generated Flue app through the Next route handler adapter', async () => {
	const root = createFixtureRoot();
	const enginePort = await getAvailablePort();
	const enginePeerPort = await getAvailablePort();
	const engineMetricsPort = await getAvailablePort();
	const instanceId = `next-${Date.now()}-${Math.random().toString(36).slice(2)}`;
	writeProject(root);

	const build = await runCli(root, ['build']);
	assert.equal(build.code, 0, build.stderr);

	const poolName = `flue-next-${Date.now()}-${Math.random().toString(36).slice(2)}`;
	const registryKey = `registry-${Date.now()}-${Math.random().toString(36).slice(2)}`;
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
	process.env.RIVET_RUN_ENGINE = '0';
	delete process.env.RIVET_RUN_ENGINE_HOST;
	delete process.env.RIVET_RUN_ENGINE_PORT;
	process.env.RIVET_ENDPOINT = `http://127.0.0.1:${enginePort}`;
	process.env.RIVET_POOL = poolName;
	process.env.FLUE_RIVET_REGISTRY_KEY = registryKey;
	delete process.env.FLUE_RIVET_ENDPOINT;
	delete process.env.NEXT_PUBLIC_SITE_URL;
	delete process.env.RIVETKIT_RUNTIME_MODE;
	delete process.env.RIVET_PUBLIC_ENDPOINT;
	delete process.env.RIVET_PUBLIC_TOKEN;

	let server;
	let engine;
	let passed = false;
	try {
		engine = startEngine({
			guardPort: enginePort,
			peerPort: enginePeerPort,
			metricsPort: engineMetricsPort,
		});
		await waitForMetadata(enginePort, engine.logs);
		server = await import(pathToFileURL(path.join(root, 'dist', 'server.mjs')).href);
		server.registry.config.test = { ...server.registry.config.test, enabled: true };
		server.registry.config.noWelcome = true;
		await configureNormalRunnerConfig(poolName, enginePort);
		await server.registry.startAndWait();

		const flueHandlers = toFlueNextHandler(server.flueApp);
		const response = await flueHandlers.POST(
			new Request(`http://next.local/api/flue/agents/assistant/${instanceId}`, {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ kind: 'user', body: 'Hello from Next' }),
			}),
			{ params: Promise.resolve({ all: ['agents', 'assistant', instanceId] }) },
		);

		const responseText = await response.text();
		assert.equal(response.status, 202, responseText);
		assert.equal(typeof JSON.parse(responseText).submissionId, 'string');
		let snapshot;
		for (let attempt = 0; attempt < 20; attempt++) {
			const history = await flueHandlers.GET(
				new Request(`http://next.local/api/flue/agents/assistant/${instanceId}?view=history`),
				{ params: Promise.resolve({ all: ['agents', 'assistant', instanceId] }) },
			);
			if (history.ok) {
				snapshot = await history.json();
				if (snapshot.messages?.some((message) =>
					message.role === 'assistant' &&
					message.parts?.some((part) => part.type === 'text' && part.state === 'done'),
				)) break;
			}
			await delay(250);
		}
		assert.ok(snapshot.messages.some((message) =>
			message.role === 'assistant' &&
			message.parts.some((part) => part.type === 'text' && part.text === 'Hello from Next route.'),
		));
		assert.equal(typeof server.registry.handler, 'function');
		assert.equal(Object.keys(server.registry.config.use).includes('flue-agent-assistant'), true);
		passed = true;
	} finally {
		if (server) await Promise.race([server.closeFlueRivetRuntime(), delay(5_000)]);
		await engine?.stop();
		killEngineOnPort(enginePort);
		restoreEnv(previousEnv);
		if (passed) setTimeout(() => process.exit(0), 100);
	}
});

function createFixtureRoot() {
	const root = fs.mkdtempSync(path.join(os.tmpdir(), 'flue-rivet-next-'));
	fixtureRoots.push(root);
	fs.mkdirSync(path.join(root, 'node_modules', '@flue'), { recursive: true });
	fs.symlinkSync(
		flueRuntimeRoot,
		path.join(root, 'node_modules', '@flue', 'runtime'),
		'dir',
	);
	fs.symlinkSync(
		flueCliRoot,
		path.join(root, 'node_modules', '@flue', 'cli'),
		'dir',
	);
	fs.mkdirSync(path.join(root, 'node_modules', '@rivet-dev'), { recursive: true });
	fs.symlinkSync(packageRoot, path.join(root, 'node_modules', '@rivet-dev', 'flue'), 'dir');
	fs.symlinkSync(path.join(packageRoot, 'node_modules', 'rivetkit'), path.join(root, 'node_modules', 'rivetkit'), 'dir');
	return root;
}

function writeProject(root) {
	fs.writeFileSync(
		path.join(root, 'package.json'),
		JSON.stringify({
			type: 'module',
			dependencies: {
				'@flue/cli': `link:${flueCliRoot}`,
				'@flue/runtime': `link:${flueRuntimeRoot}`,
				'@rivet-dev/flue': `link:${packageRoot}`,
				rivetkit: `link:${path.join(packageRoot, 'node_modules', 'rivetkit')}`,
			},
		}),
	);
	fs.writeFileSync(
		path.join(root, 'flue.config.ts'),
		`import { defineConfig } from '@flue/cli/config';\nimport rivet from '@rivet-dev/flue';\nexport default defineConfig({ target: rivet });\n`,
	);
	fs.mkdirSync(path.join(root, 'agents'), { recursive: true });
	fs.writeFileSync(
		path.join(root, 'agents', 'assistant.ts'),
		`import { createAgent, registerProvider } from '@flue/runtime';\nimport { fauxAssistantMessage, registerFauxProvider } from '@flue/runtime/adapter-kit';\nexport const route = async (_c, next) => next();\nconst provider = registerFauxProvider({ provider: 'rivet-next-' + crypto.randomUUID() });\nprovider.setResponses([fauxAssistantMessage('Hello from Next route.')]);\nconst model = provider.getModel();\nregisterProvider(model.provider, { api: provider.api, baseUrl: model.baseUrl });\nexport default createAgent(() => ({ model: model.provider + '/' + model.id }));\n`,
	);
}

async function runCli(cwd, args) {
	const child = spawn(process.execPath, [cli.pathname, ...args], {
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

async function getAvailablePort() {
	const server = createServer();
	server.listen(0, '127.0.0.1');
	await once(server, 'listening');
	const address = server.address();
	assert(address && typeof address === 'object');
	server.close();
	await once(server, 'close');
	return address.port;
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

function delay(ms) {
	return new Promise((resolve) => setTimeout(resolve, ms));
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
