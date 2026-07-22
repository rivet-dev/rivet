import assert from 'node:assert/strict';
import { DatabaseSync } from 'node:sqlite';
import { describe, it } from 'node:test';
import {
	createAsyncSqlStores,
	ensureAsyncSqlSchema,
} from '../src/index.ts';

describe('rivet async stores', () => {
	it('covers sessions, submissions, recovery helpers, runs, and streams when used through the public stores', async () => {
		const db = new TestSqliteDb();
		await ensureAsyncSqlSchema(db);
		const stores = createAsyncSqlStores(db);
		const { sessions, submissions } = stores.executionStore;

		await sessions.save('session-1', {
			entries: [
				{
					id: 'entry-1',
					type: 'message',
					message: {
						role: 'user',
						content: [
							{ type: 'text', text: 'hello' },
							{ type: 'image', data: 'a'.repeat(300_000), mimeType: 'image/png' },
						],
					},
				},
			],
			summary: { message: 'summary' },
		});
		const loaded = await sessions.load('session-1');
		assert.equal(loaded.entries.length, 1);
		assert.equal(loaded.entries[0].message.content[1].data, 'a'.repeat(300_000));
		await sessions.delete('session-1');
		assert.equal(await sessions.load('session-1'), null);

		const direct = directInput('direct-1', 'agent-1', 'instance-1', 'hello');
		const admitted = await submissions.admitDirect(direct);
		assert.equal(admitted.status, 'queued');
		assert.equal((await submissions.admitDirect(direct)).submissionId, 'direct-1');
		assert.equal(await submissions.hasUnsettledSubmissions(), true);
		assert.deepEqual(
			(await submissions.listRunnableSubmissions()).map((submission) => submission.submissionId),
			['direct-1'],
		);

		const claim = await submissions.claimSubmission({
			submissionId: 'direct-1',
			attemptId: 'attempt-1',
			ownerId: 'owner-1',
			leaseExpiresAt: Date.now() - 1,
		});
		assert.equal(claim.attemptId, 'attempt-1');
		assert.equal(await submissions.claimSubmission({
			submissionId: 'direct-1',
			attemptId: 'attempt-other',
			ownerId: 'owner-2',
			leaseExpiresAt: Date.now() + 10_000,
		}), null);
		assert.equal((await submissions.listRunningSubmissions()).length, 1);
		assert.equal((await submissions.listExpiredSubmissions()).length, 1);
		await submissions.renewLeases('owner-1', ['direct-1']);
		assert.equal((await submissions.listExpiredSubmissions()).length, 0);

		assert.equal(
			await submissions.beginTurnJournal({
				submissionId: 'direct-1',
				sessionKey: admitted.sessionKey,
				kind: 'direct',
				attemptId: 'attempt-1',
				operationId: 'operation-1',
				turnId: 'turn-1',
				phase: 'before_provider',
				checkpointLeafId: 'leaf-0',
			}),
			true,
		);
		assert.equal(
			await submissions.updateTurnJournalPhase(
				{ submissionId: 'direct-1', attemptId: 'attempt-1' },
				'provider_started',
				{ toolRequest: { name: 'tool' }, streamKey: 'stream-1' },
			),
			true,
		);
		const journal = await submissions.getTurnJournal('direct-1');
		assert.equal(journal.phase, 'provider_started');
		assert.deepEqual(journal.toolRequest, { name: 'tool' });

		assert.equal(await submissions.appendStreamChunkSegment('stream-1', 0, 'hel'), true);
		assert.equal(await submissions.appendStreamChunkSegment('stream-1', 0, 'ignored'), false);
		assert.equal(await submissions.appendStreamChunkSegment('stream-1', 1, 'lo'), true);
		assert.deepEqual(await submissions.getStreamChunkSegments('stream-1'), [
			{ segmentIndex: 0, body: 'hel' },
			{ segmentIndex: 1, body: 'lo' },
		]);
		assert.equal(
			await submissions.markStreamConsumed(
				{ submissionId: 'direct-1', attemptId: 'attempt-1' },
				'stream-1',
			),
			true,
		);
		assert.equal(
			await submissions.markStreamConsumed(
				{ submissionId: 'direct-1', attemptId: 'attempt-1' },
				'stream-1',
			),
			false,
		);
		await submissions.deleteStreamChunkSegments('stream-1');
		assert.deepEqual(await submissions.getStreamChunkSegments('stream-1'), []);

		assert.equal(
			await submissions.markSubmissionInputApplied(
				{ submissionId: 'direct-1', attemptId: 'attempt-1' },
				{ maxRetry: 3, timeoutAt: Date.now() + 60_000 },
			),
			true,
		);
		assert.equal(
			await submissions.requestSubmissionRecovery({
				submissionId: 'direct-1',
				attemptId: 'attempt-1',
			}),
			true,
		);
		const replaced = await submissions.replaceTurnJournalAttempt(
			{ submissionId: 'direct-1', attemptId: 'attempt-1' },
			'attempt-2',
			{ ownerId: 'owner-2', leaseExpiresAt: Date.now() + 10_000 },
		);
		assert.equal(replaced.attemptId, 'attempt-2');
		assert.equal((await submissions.getTurnJournal('direct-1')).attemptId, 'attempt-2');
		assert.equal(
			await submissions.commitTurnJournal(
				{ submissionId: 'direct-1', attemptId: 'attempt-2' },
				'leaf-1',
			),
			true,
		);
		assert.equal(
			await submissions.completeSubmission({ submissionId: 'direct-1', attemptId: 'attempt-2' }),
			true,
		);
		assert.equal(await submissions.hasUnsettledSubmissions(), false);

		await submissions.insertAttemptMarker({ submissionId: 'direct-1', attemptId: 'attempt-2' });
		await submissions.insertAttemptMarker({ submissionId: 'direct-1', attemptId: 'attempt-2' });
		assert.equal((await submissions.listAttemptMarkers()).length, 1);
		await submissions.deleteAttemptMarker({ submissionId: 'direct-1', attemptId: 'attempt-2' });
		assert.equal((await submissions.listAttemptMarkers()).length, 0);

		const dispatch = dispatchInput('dispatch-1', 'agent-1', 'instance-2');
		const dispatchAdmission = await submissions.admitDispatch(dispatch);
		assert.equal(dispatchAdmission.kind, 'submission');
		const dispatchClaim = await submissions.claimSubmission({
			submissionId: 'dispatch-1',
			attemptId: 'dispatch-attempt-1',
			ownerId: 'owner-1',
			leaseExpiresAt: Date.now() + 10_000,
		});
		assert.equal(dispatchClaim.submissionId, 'dispatch-1');
		assert.equal(
			await submissions.requeueSubmissionBeforeInputApplied({
				submissionId: 'dispatch-1',
				attemptId: 'dispatch-attempt-1',
			}),
			true,
		);
		const dispatchClaim2 = await submissions.claimSubmission({
			submissionId: 'dispatch-1',
			attemptId: 'dispatch-attempt-2',
			ownerId: 'owner-1',
			leaseExpiresAt: Date.now() + 10_000,
		});
		assert.equal(dispatchClaim2.attemptId, 'dispatch-attempt-2');
		assert.equal(
			await submissions.failSubmission(
				{ submissionId: 'dispatch-1', attemptId: 'dispatch-attempt-2' },
				new Error('boom'),
			),
			true,
		);
		assert.equal((await submissions.getSubmission('dispatch-1')).error, 'boom');

		await submissions.deleteSession(dispatchAdmission.submission.sessionKey, async () => {});
		assert.deepEqual(await submissions.listPendingSessionDeletions(), []);
		const retained = await submissions.admitDispatch(dispatch);
		assert.equal(retained.kind, 'retained_receipt');

		await stores.runStore.createRun({
			runId: 'run-1',
			workflowName: 'workflow',
			startedAt: '2026-06-17T00:00:00.000Z',
			payload: { hello: true },
		});
		await stores.runStore.createRun({
			runId: 'run-2',
			workflowName: 'workflow',
			startedAt: '2026-06-17T00:00:01.000Z',
			payload: null,
		});
		await stores.runStore.endRun({
			runId: 'run-1',
			endedAt: '2026-06-17T00:00:02.000Z',
			isError: false,
			durationMs: 2000,
			result: { ok: true },
		});
		assert.equal((await stores.runStore.getRun('run-1')).status, 'completed');
		assert.equal((await stores.runStore.lookupRun('run-1')).workflowName, 'workflow');
		const firstPage = await stores.runStore.listRuns({ workflowName: 'workflow', limit: 1 });
		assert.equal(firstPage.runs[0].runId, 'run-2');
		assert.ok(firstPage.nextCursor);
		const secondPage = await stores.runStore.listRuns({
			workflowName: 'workflow',
			limit: 1,
			cursor: firstPage.nextCursor,
		});
		assert.equal(secondPage.runs[0].runId, 'run-1');

		let notifications = 0;
		await stores.eventStreamStore.createStream('runs/run-1');
		const unsubscribe = stores.eventStreamStore.subscribe('runs/run-1', () => notifications++);
		assert.equal(await stores.eventStreamStore.appendEvent('runs/run-1', { type: 'start' }), '0000000000000000_0000000000000000');
		assert.equal(await stores.eventStreamStore.appendEvent('runs/run-1', { type: 'end' }), '0000000000000000_0000000000000001');
		assert.equal(notifications, 2);
		const streamRead = await stores.eventStreamStore.readEvents('runs/run-1', { limit: 1 });
		assert.equal(streamRead.events.length, 1);
		assert.equal(streamRead.upToDate, false);
		await stores.eventStreamStore.closeStream('runs/run-1');
		assert.equal((await stores.eventStreamStore.getStreamMeta('runs/run-1')).closed, true);
		unsubscribe();
	});

	it('keeps direct admission atomic when another call interleaves at an awaited query', async () => {
		const db = new TestSqliteDb();
		await ensureAsyncSqlSchema(db);
		const stores = createAsyncSqlStores(db);
		const releaseInsert = createDeferred();
		let paused = false;
		db.afterQuery = async (text, inTransaction) => {
			if (
				inTransaction &&
				!paused &&
				text.includes('INSERT OR IGNORE INTO flue_agent_submissions')
			) {
				paused = true;
				await releaseInsert.promise;
			}
		};

		const admission = stores.executionStore.submissions.admitDirect(
			directInput('atomic-1', 'agent-1', 'atomic-instance', 'hello'),
		);
		while (!paused) await new Promise((resolve) => setTimeout(resolve, 0));

		let readSettled = false;
		const interleavedRead = stores.executionStore.submissions
			.getSubmission('atomic-1')
			.then((submission) => {
				readSettled = true;
				return submission;
			});
		await new Promise((resolve) => setTimeout(resolve, 20));
		assert.equal(readSettled, false);

		releaseInsert.resolve();
		const admitted = await admission;
		const read = await interleavedRead;
		assert.equal(read.submissionId, admitted.submissionId);
		assert.deepEqual(read.input, admitted.input);
	});
});

class TestSqliteDb {
	db = new DatabaseSync(':memory:');
	tail = Promise.resolve();
	afterQuery = undefined;

	async query(text, params = []) {
		await this.tail;
		return this.queryDirect(text, params, false);
	}

	async transaction(fn) {
		const previous = this.tail;
		let release;
		this.tail = new Promise((resolve) => {
			release = resolve;
		});
		await previous;
		this.db.exec('BEGIN');
		try {
			const result = await fn({
				query: (text, params = []) => this.queryDirect(text, params, true),
			});
			this.db.exec('COMMIT');
			return result;
		} catch (error) {
			this.db.exec('ROLLBACK');
			throw error;
		} finally {
			release();
		}
	}

	async queryDirect(text, params = [], inTransaction) {
		const rows = this.db.prepare(text).all(...params.map(sqliteValue));
		if (this.afterQuery) await this.afterQuery(text, inTransaction);
		return rows.map((row) => ({ ...row }));
	}
}

function sqliteValue(value) {
	if (typeof value === 'boolean') return value ? 1 : 0;
	return value;
}

function directInput(submissionId, agent, id, message) {
	return {
		kind: 'direct',
		submissionId,
		agent,
		id,
		payload: {
			message,
			images: [{ type: 'image', data: 'b'.repeat(300_000), mimeType: 'image/png' }],
		},
		acceptedAt: '2026-06-17T00:00:00.000Z',
	};
}

function dispatchInput(dispatchId, agent, id) {
	return {
		dispatchId,
		agent,
		id,
		input: { value: 1 },
		acceptedAt: '2026-06-17T00:00:00.000Z',
	};
}

function createDeferred() {
	let resolve;
	const promise = new Promise((innerResolve) => {
		resolve = innerResolve;
	});
	return { promise, resolve };
}
