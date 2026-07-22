import assert from 'node:assert/strict';
import { DatabaseSync } from 'node:sqlite';
import { afterEach, describe, it } from 'node:test';
import { createAgent } from '@flue/runtime';
import {
	createFlueContext,
	fauxAssistantMessage,
	registerFauxProvider,
} from '@flue/runtime/adapter-kit';
import {
	createAsyncEventStreamStore,
	createRivetAgentRuntime,
	ensureAsyncSqlSchema,
	RIVET_AGENT_INTERNAL_DISPATCH_PATH,
} from '../src/index.ts';

const providers = [];

afterEach(() => {
	for (const provider of providers.splice(0)) provider.unregister();
});

describe('RivetAgentCoordinator', () => {
	it('round-trips an attached prompt through durable admission and event streaming', async () => {
		const provider = createProvider();
		provider.setResponses([fauxAssistantMessage('Rivet reply.')]);
		const host = await createHost(provider);

		const response = await host.runtime.onRequest(
			host.actor,
			new Request('http://flue.local/agents/assistant/instance-1?wait=result', {
				method: 'POST',
				headers: { 'content-type': 'application/json' },
				body: JSON.stringify({ message: 'Hello' }),
			}),
		);

		assert.equal(response.status, 200);
		const body = await response.json();
		assert.equal(body.result.text, 'Rivet reply.');
		assert.equal((await host.stores.executionStore.submissions.hasUnsettledSubmissions()), false);

		const stream = await host.stores.eventStreamStore.readEvents('agents/assistant/instance-1');
		assert.ok(stream.events.some((event) => event.data.type === 'agent_start'));
		assert.ok(stream.events.some((event) => event.data.type === 'agent_end'));
	});

	it('recovers an input-applied interrupted submission from onWake', async () => {
		const provider = createProvider();
		provider.setResponses([fauxAssistantMessage('Recovered after wake.')]);
		const host = await createHost(provider);
		const input = dispatchInput('dispatch-recover', 'assistant', 'instance-1');

		await host.stores.executionStore.submissions.admitDispatch(input);
		await host.stores.executionStore.submissions.claimSubmission({
			submissionId: input.dispatchId,
			attemptId: 'attempt-interrupted',
			ownerId: 'old-actor',
			leaseExpiresAt: 0,
		});
		await host.stores.executionStore.submissions.markSubmissionInputApplied({
			submissionId: input.dispatchId,
			attemptId: 'attempt-interrupted',
		});
		await host.stores.executionStore.sessions.save(sessionKey('instance-1'), {
			version: 6,
			affinityKey: 'aff_01J00000000000000000000000',
			taskSessions: [],
			entries: [
				{
					type: 'message',
					id: 'entry-1',
					parentId: null,
					timestamp: new Date().toISOString(),
					message: { role: 'user', content: 'Hello', timestamp: Date.now() },
					dispatch: { dispatchId: input.dispatchId },
				},
			],
			leafId: 'entry-1',
			metadata: {},
			createdAt: new Date().toISOString(),
			updatedAt: new Date().toISOString(),
		});

		const restarted = await createHost(provider, host.db);
		await restarted.runtime.onWake(restarted.actor);
		await restarted.actor.waitForKeepAwake();

		const submission = await restarted.stores.executionStore.submissions.getSubmission(
			input.dispatchId,
		);
		assert.equal(submission.status, 'settled');
		assert.equal(submission.error, undefined);
		const session = await restarted.stores.executionStore.sessions.load(sessionKey('instance-1'));
		const assistant = session.entries.findLast(
			(entry) => entry.type === 'message' && entry.message.role === 'assistant',
		);
		assert.equal(assistant.message.content[0].text, 'Recovered after wake.');
	});

	it('resumes pending session deletion from onWake', async () => {
		const provider = createProvider();
		const host = await createHost(provider);
		const key = sessionKey('instance-1');
		await host.stores.executionStore.sessions.save(key, {
			version: 6,
			affinityKey: 'aff_01J00000000000000000000000',
			taskSessions: [],
			entries: [],
			leafId: null,
			metadata: {},
			createdAt: new Date().toISOString(),
			updatedAt: new Date().toISOString(),
		});
		await host.db.query(
			'INSERT INTO flue_agent_session_deletions (session_key, started_at) VALUES (?, ?)',
			[key, Date.now()],
		);

		await host.runtime.onWake(host.actor);

		assert.deepEqual(await host.stores.executionStore.submissions.listPendingSessionDeletions(), []);
		assert.equal(await host.stores.executionStore.sessions.load(key), null);
	});

	describe('internal dispatch admission', () => {
		it('returns 400 when the dispatch targets a different agent instance', async () => {
			const host = await createHost(createProvider());

			const response = await host.runtime.onRequest(
				host.actor,
				dispatchRequest({
					dispatchId: 'dispatch-wrong-target',
					agent: 'assistant',
					id: 'instance-2',
					input: { message: 'Hello' },
					acceptedAt: new Date().toISOString(),
				}),
			);

			assert.equal(response.status, 400);
			assert.equal(
				await host.stores.executionStore.submissions.getSubmission('dispatch-wrong-target'),
				null,
			);
		});

		it('returns 404 when the target agent is not registered', async () => {
			const host = await createHost(createProvider(), new TestSqliteDb(), { registerAgent: false });

			const response = await host.runtime.onRequest(
				host.actor,
				dispatchRequest({
					dispatchId: 'dispatch-missing-agent',
					agent: 'assistant',
					id: 'instance-1',
					input: { message: 'Hello' },
					acceptedAt: new Date().toISOString(),
				}),
			);

			assert.equal(response.status, 404);
		});

		it('returns 409 when the same dispatch id replays with a conflicting payload', async () => {
			const host = await createHost(createProvider());
			await host.stores.executionStore.submissions.admitDispatch(
				dispatchInput('dispatch-replay', 'assistant', 'instance-1'),
			);

			const response = await host.runtime.onRequest(
				host.actor,
				dispatchRequest({
					dispatchId: 'dispatch-replay',
					agent: 'assistant',
					id: 'instance-1',
					input: { message: 'A different message' },
					acceptedAt: new Date().toISOString(),
				}),
			);

			assert.equal(response.status, 409);
		});

		it('returns the retained receipt and prior dispatch id when one already exists', async () => {
			const host = await createHost(createProvider());
			const acceptedAt = Date.parse('2026-01-01T00:00:00.000Z');
			await host.db.query(
				'INSERT INTO flue_agent_dispatch_receipts (dispatch_id, accepted_at) VALUES (?, ?)',
				['dispatch-retained', acceptedAt],
			);

			const response = await host.runtime.onRequest(
				host.actor,
				dispatchRequest({
					dispatchId: 'dispatch-retained',
					agent: 'assistant',
					id: 'instance-1',
					input: { message: 'Hello' },
					acceptedAt: new Date().toISOString(),
				}),
			);

			assert.equal(response.status, 200);
			const body = await response.json();
			assert.equal(body.dispatchId, 'dispatch-retained');
			assert.equal(body.acceptedAt, '2026-01-01T00:00:00.000Z');
			// Retained receipts must not create a fresh queued submission.
			assert.equal(
				await host.stores.executionStore.submissions.getSubmission('dispatch-retained'),
				null,
			);
		});
	});
});

async function createHost(provider, db = new TestSqliteDb(), { registerAgent = true } = {}) {
	await ensureAsyncSqlSchema(db);
	const actor = new FakeActor(db);
	const agent = createAgent(() => ({
		model: `${provider.getModel().provider}/${provider.getModel().id}`,
	}));
	const runtime = createRivetAgentRuntime({
		// When registerAgent is false the coordinator is prepared for 'assistant'
		// but no created agent is registered, exercising the dispatch 404 branch.
		createdAgents: registerAgent ? { assistant: agent } : {},
		createEventStreamStore: (actorContext) => createAsyncEventStreamStore(actorContext.db),
		createContext: ({ executionStore, actor, payload, request, initialEventIndex, dispatchId }) =>
			createFlueContext({
				id: actor.key[0],
				runId: undefined,
				dispatchId,
				payload,
				env: actor.env ?? {},
				req: request,
				initialEventIndex,
				agentConfig: {
					subagents: {},
					resolveModel: () => provider.getModel(),
				},
				createDefaultEnv: async () => createNoopSessionEnv(),
				defaultStore: executionStore.sessions,
				submissionStore: executionStore.submissions,
			}),
	});
	const prepared = await runtime.prepare({ db, agentName: 'assistant' });
	runtime.attach(actor, prepared);
	return {
		db,
		actor,
		runtime,
		stores: {
			executionStore: prepared.executionStore,
			eventStreamStore: createAsyncEventStreamStore(db),
		},
	};
}

function createProvider() {
	const provider = registerFauxProvider({ provider: `rivet-test-${crypto.randomUUID()}` });
	providers.push(provider);
	return provider;
}

class FakeActor {
	actorId = crypto.randomUUID();
	name = 'assistant';
	key = ['instance-1'];
	region = 'local';
	env = {};
	scheduled = [];
	abortController = new AbortController();
	keepAwakePromises = new Set();

	constructor(db) {
		this.db = db;
	}

	get abortSignal() {
		return this.abortController.signal;
	}

	async keepAwake(promise) {
		this.keepAwakePromises.add(promise);
		try {
			return await promise;
		} finally {
			this.keepAwakePromises.delete(promise);
		}
	}

	schedule = {
		after: async (duration, action, ...args) => {
			this.scheduled.push({ duration, action, args });
		},
	};

	async waitForKeepAwake() {
		while (this.keepAwakePromises.size > 0) {
			await Promise.allSettled([...this.keepAwakePromises]);
		}
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

function sessionKey(instanceId) {
	return `agent-session:${JSON.stringify([instanceId, 'default', 'default'])}`;
}

function dispatchInput(dispatchId, agent, id) {
	return {
		dispatchId,
		agent,
		id,
		input: { message: 'Hello' },
		acceptedAt: new Date().toISOString(),
	};
}

function dispatchRequest(body) {
	return new Request(`http://flue.local${RIVET_AGENT_INTERNAL_DISPATCH_PATH}`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify(body),
	});
}
