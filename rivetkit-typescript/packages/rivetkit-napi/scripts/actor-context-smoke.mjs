import assert from "node:assert/strict";

import { ActorContext } from "../index.js";

async function main() {
	const ctx = new ActorContext("actor-smoke", "smoke", "local");

	const signal = ctx.abortSignal();
	assert.equal(signal instanceof AbortSignal, true);
	assert.equal(signal.aborted, false);
}

await main();
