import { setupTest } from "rivetkit/test";
import { expect, test, vi } from "vitest";
import { registry } from "../src/backend/registry";

test("Rate limiter allows requests under limit", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const limiter = client.rateLimiter.getOrCreate(["test-limit"]);

	// Test first request - should be allowed
	const result1 = await limiter.checkLimit();
	expect(result1).toMatchObject({
		allowed: true,
		remaining: 4, // 5 total - 1 used = 4 remaining
		resetsIn: expect.any(Number),
	});

	// Test additional requests
	const result2 = await limiter.checkLimit();
	expect(result2.allowed).toBe(true);
	expect(result2.remaining).toBe(3);

	const result3 = await limiter.checkLimit();
	expect(result3.allowed).toBe(true);
	expect(result3.remaining).toBe(2);
});

test("Rate limiter blocks requests over limit", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const limiter = client.rateLimiter.getOrCreate(["test-block"]);

	// Use up all 5 requests
	for (let i = 0; i < 5; i++) {
		const result = await limiter.checkLimit();
		expect(result.allowed).toBe(true);
	}

	// 6th request should be blocked
	const blocked = await limiter.checkLimit();
	expect(blocked.allowed).toBe(false);
	expect(blocked.remaining).toBe(0);
	expect(blocked.resetsIn).toBeGreaterThan(0);

	// 7th request should also be blocked
	const blocked2 = await limiter.checkLimit();
	expect(blocked2.allowed).toBe(false);
	expect(blocked2.remaining).toBe(0);
});

test("Rate limiter status reflects current state", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const limiter = client.rateLimiter.getOrCreate(["test-status"]);

	// Initial status
	const initial = await limiter.getStatus();
	expect(initial).toMatchObject({
		count: 0,
		remaining: 5,
		resetsIn: 0, // No reset time set yet
	});

	// After some requests
	await limiter.checkLimit();
	await limiter.checkLimit();

	const afterRequests = await limiter.getStatus();
	expect(afterRequests.count).toBe(2);
	expect(afterRequests.remaining).toBe(3);
	expect(afterRequests.resetsIn).toBeGreaterThan(0);
});

test("Rate limiter reset functionality", async (ctx) => {
	const { client } = await setupTest(ctx, registry);
	const limiter = client.rateLimiter.getOrCreate(["test-reset"]);

	// Use up some requests
	await limiter.checkLimit();
	await limiter.checkLimit();
	await limiter.checkLimit();

	const beforeReset = await limiter.getStatus();
	expect(beforeReset.count).toBe(3);
	expect(beforeReset.remaining).toBe(2);

	// Reset the limiter
	const resetResult = await limiter.reset();
	expect(resetResult.success).toBe(true);

	// Check status after reset
	const afterReset = await limiter.getStatus();
	expect(afterReset.count).toBe(0);
	expect(afterReset.remaining).toBe(5);
	expect(afterReset.resetsIn).toBe(0);

	// Should be able to make requests again
	const newRequest = await limiter.checkLimit();
	expect(newRequest.allowed).toBe(true);
	expect(newRequest.remaining).toBe(4);
});
