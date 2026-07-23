import assert from 'node:assert/strict';
import { DatabaseSync } from 'node:sqlite';
import { describe, it } from 'node:test';
import { defineAgent, defineWorkflow } from '@flue/runtime';
import { createFlueContext, registerFauxProvider } from '@flue/runtime/adapter-kit';
import {
	createAsyncEventStreamStore,
	createAsyncRegistryOps,
	createAsyncRunStore,
	createRivetWorkflowRuntime,
	ensureAsyncSqlSchema,
} from '../src/index.ts';

describe('RivetWorkflowCoordinator', () => {
	it('runs a workflow and mirrors the run into the registry when requested', async () => {
		const registryDb = new TestSqliteDb();
		const registry = await createAsyncRegistryOps(registryDb);
		const host = await createHost({
			registry,
			handler: async () => ({ ok: true }),
		});

		const response = await host.runtime.onRequest(
			host.actor,
			new Request('http://flue.local/workflows/job?wait=result', {
				method: 'POST',
			}),
		);

		assert.equal(response.status, 200);
		const body = await response.json();
		assert.equal(body.runId, 'run-1');
		assert.deepEqual(body.result, { ok: true });
		const pointer = await registry.lookupRun('run-1');
		assert.equal(pointer.workflowName, 'job');
		assert.equal((await registry.getRun('run-1')).status, 'completed');
		assert.equal((await registry.listRuns()).runs[0].runId, 'run-1');

		const meta = await host.runtime.onRequest(
			host.actor,
			new Request('http://flue.local/runs/run-1?meta'),
		);
		assert.equal(meta.status, 200);
		assert.equal((await meta.json()).status, 'completed');

		const stream = await host.stores.eventStreamStore.readEvents('runs/run-1');
		assert.ok(stream.events.some((event) => event.data.type === 'run_start'));
		assert.ok(stream.events.some((event) => event.data.type === 'run_end'));
	});

	it('terminalizes an active run from onWake when a workflow actor restarts', async () => {
		const db = new TestSqliteDb();
		await ensureAsyncSqlSchema(db);
		const runStore = createAsyncRunStore(db);
		await runStore.createRun({
			runId: 'run-1',
			workflowName: 'job',
			startedAt: new Date().toISOString(),
			input: { value: 1 },
		});

		const host = await createHost({ db, handler: async () => 'unused' });
		await host.runtime.onWake(host.actor);

		const recovered = await runStore.getRun('run-1');
		assert.equal(recovered.status, 'errored');
		const stream = await host.stores.eventStreamStore.readEvents('runs/run-1');
		assert.ok(stream.closed);
		assert.ok(stream.events.some((event) => event.data.type === 'run_resume'));
		assert.ok(stream.events.some((event) => event.data.type === 'run_end' && event.data.isError));
	});
});

async function createHost({ db = new TestSqliteDb(), registry, handler }) {
	await ensureAsyncSqlSchema(db);
	const actor = new FakeActor(db);
	const provider = registerFauxProvider({ provider: `workflow-test-${crypto.randomUUID()}` });
	const model = provider.getModel();
	const workflowAgent = defineAgent(() => ({ model: `${model.provider}/${model.id}` }));
	const workflow = defineWorkflow({ agent: workflowAgent, run: handler });
	const runtime = createRivetWorkflowRuntime({
		workflows: [{ name: 'job', definition: workflow }],
		createEventStreamStore: (actorContext) => createAsyncEventStreamStore(actorContext.db),
		createRegistryRunStore: () => registry?.runStore,
		createContext: ({ actor, request, runId, initialEventIndex }) => {
			return createFlueContext({
				id: runId,
				runId,
				env: actor.env ?? {},
				req: request,
				initialEventIndex,
				agentConfig: {
					resolveModel: () => model,
				},
				createDefaultEnv: async () => createNoopSessionEnv(),
			});
		},
	});
	const prepared = await runtime.prepare({ db, workflowName: 'job', actor });
	runtime.attach(actor, prepared);
	return {
		db,
		actor,
		runtime,
		stores: {
			runStore: prepared.runStore,
			eventStreamStore: createAsyncEventStreamStore(db),
		},
	};
}

class FakeActor {
	actorId = crypto.randomUUID();
	name = 'job';
	key = ['run-1'];
	region = 'local';
	env = {};
	abortController = new AbortController();

	constructor(db) {
		this.db = db;
	}

	get abortSignal() {
		return this.abortController.signal;
	}

	async keepAwake(promise) {
		return promise;
	}
}

class TestSqliteDb {
	db = new DatabaseSync(':memory:');
	tail = Promise.resolve();

	async query(text, params = []) {
		await this.tail;
		return this.queryDirect(text, params);
	}

	async transaction(fn) {
		const previous = this.tail;
		let release;
		this.tail = new Promise((resolve) => {
			release = resolve;
		});
		await previous;
		this.db.exec('BEGIN IMMEDIATE');
		try {
			const result = await fn({ query: (text, params = []) => this.queryDirect(text, params) });
			this.db.exec('COMMIT');
			return result;
		} catch (error) {
			this.db.exec('ROLLBACK');
			throw error;
		} finally {
			release();
		}
	}

	async queryDirect(text, params = []) {
		const rows = this.db.prepare(text).all(...params.map(sqliteValue));
		return rows.map((row) => ({ ...row }));
	}
}

function createNoopSessionEnv() {
	const cwd = '/repo';
	const resolvePath = (path) => normalizePath(path.startsWith('/') ? path : `${cwd}/${path}`);
	return {
		cwd,
		resolvePath,
		exec: async () => ({ stdout: '', stderr: '', exitCode: 0 }),
		readFile: async () => '',
		readFileBuffer: async () => new Uint8Array(),
		writeFile: async () => {},
		stat: async () => ({ isFile: false, isDirectory: false }),
		readdir: async () => [],
		exists: async () => false,
		mkdir: async () => {},
		rm: async () => {},
	};
}

function normalizePath(path) {
	const segments = [];
	for (const segment of path.split('/')) {
		if (!segment || segment === '.') continue;
		if (segment === '..') segments.pop();
		else segments.push(segment);
	}
	return `/${segments.join('/')}`;
}

function sqliteValue(value) {
	if (typeof value === 'boolean') return value ? 1 : 0;
	return value;
}
