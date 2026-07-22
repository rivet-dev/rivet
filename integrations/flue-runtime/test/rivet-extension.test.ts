import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import {
	extend,
	markFlueRivetActorDefinition,
	resolveRivetExtension,
} from '../src/extension.ts';

describe('Rivet extension API', () => {
	it('resolves an extension when it preserves the Flue actor config marker', () => {
		const base = markFlueRivetActorDefinition({ config: { name: 'assistant' } });
		const rivet = extend((definition) => ({
			...definition,
			config: definition.config,
			extra: true,
		}));

		const resolved = resolveRivetExtension({ rivet }, 'assistant', 'Agent');
		const output = resolved(base);

		assert.equal(output.extra, true);
	});

	it('rejects an extension that replaces the Flue actor config marker', () => {
		const base = markFlueRivetActorDefinition({ config: { name: 'assistant' } });
		const rivet = extend((definition) => ({
			...definition,
			config: { name: 'assistant' },
		}));

		const resolved = resolveRivetExtension({ rivet }, 'assistant', 'Agent');

		assert.throws(() => resolved(base), /preserves definition\.config/);
	});
});
