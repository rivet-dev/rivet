import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

describe('@rivet-dev/flue entrypoints', () => {
	it('resolves the runtime and internal subpaths without exporting the target', async () => {
		const root = await import('@rivet-dev/flue');
		const runtime = await import('@rivet-dev/flue/runtime');
		const internal = await import('@rivet-dev/flue/internal');

		assert.equal(typeof root.createRivetAgentRuntime, 'function');
		assert.equal('rivet' in root, false);
		assert.equal('default' in root, false);
		assert.equal(typeof runtime.createRivetAgentRuntime, 'function');
		assert.equal('flueRegistry' in runtime, false);
		assert.equal(typeof internal.actor, 'function');
	});
});
