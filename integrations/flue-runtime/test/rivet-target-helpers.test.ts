import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { isTransientRivetStartError, retryTransientRivetStart } from '../src/index.ts';

describe('isTransientRivetStartError()', () => {
	it('classifies known Rivet actor-start errors as transient', () => {
		assert.equal(isTransientRivetStartError(new Error('no_envoys available')), true);
		assert.equal(isTransientRivetStartError(new Error('actor_ready_timeout')), true);
		assert.equal(isTransientRivetStartError('fetch failed'), true);
	});

	it('classifies arbitrary errors as non-transient', () => {
		assert.equal(isTransientRivetStartError(new Error('boom')), false);
		assert.equal(isTransientRivetStartError(new Error('invalid request')), false);
	});
});

describe('retryTransientRivetStart()', () => {
	it('returns the result without retrying when the action succeeds', async () => {
		let calls = 0;
		const result = await retryTransientRivetStart(async () => {
			calls++;
			return 'ok';
		});
		assert.equal(result, 'ok');
		assert.equal(calls, 1);
	});

	it('rethrows a non-transient error immediately without retrying', async () => {
		let calls = 0;
		await assert.rejects(
			retryTransientRivetStart(async () => {
				calls++;
				throw new Error('permanent failure');
			}),
			/permanent failure/,
		);
		assert.equal(calls, 1);
	});
});
