import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

describe('@rivet-dev/flue entrypoints', () => {
	it('resolves the target, runtime, internal, next, and extension subpaths', async () => {
		const root = await import('@rivet-dev/flue');
		const runtime = await import('@rivet-dev/flue/runtime');
		const internal = await import('@rivet-dev/flue/internal');
		const next = await import('@rivet-dev/flue/next');
		const extension = await import('@rivet-dev/flue/extension');

		assert.equal(root.default.name, 'rivet');
		assert.equal(typeof runtime.createRivetAgentRuntime, 'function');
		assert.equal(typeof internal.actor, 'function');
		assert.equal(typeof next.toFlueNextHandler, 'function');
		assert.equal(typeof extension.extend, 'function');
	});
});
