import assert from "node:assert/strict";

import { ActorContext } from "../index.js";

async function main() {
	const ctx = new ActorContext("actor-smoke", "smoke", "local");

	assert.throws(() => ctx.markStarted(), /ready/i);

	ctx.markReady();
	ctx.markReady();
	ctx.markStarted();
	ctx.markStarted();

	const signal = ctx.abortSignal();
	assert.equal(signal instanceof AbortSignal, true);
	assert.equal(signal.aborted, false);
}

await main();
