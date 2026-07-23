import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

describe('@rivet-dev/flue-target entrypoints', () => {
	it('resolves the target and generated-runtime subpath', async () => {
		const target = await import('@rivet-dev/flue-target');
		const runtime = await import('@rivet-dev/flue-target/runtime');

		assert.equal(target.default.name, 'rivet');
		assert.equal(typeof target.rivet, 'function');
		assert.equal(typeof runtime.createRivetAgentRuntime, 'function');
	});
});
