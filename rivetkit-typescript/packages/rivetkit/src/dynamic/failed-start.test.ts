import { describe, expect, test, vi } from "vitest";
import {
	createDynamicRuntimeStatus,
	coalesceDynamicStartup,
	transitionToInactive,
	SANITIZED_STARTUP_MESSAGE,
} from "./mod";
import type { DynamicStartupOptions } from "./mod";

const TEST_OPTIONS: Required<DynamicStartupOptions> = {
	timeoutMs: 500,
	retryInitialDelayMs: 100,
	retryMaxDelayMs: 5000,
	retryMultiplier: 2,
	retryJitter: false,
	maxAttempts: 3,
};

function opts(
	overrides?: Partial<Required<DynamicStartupOptions>>,
): Required<DynamicStartupOptions> {
	return { ...TEST_OPTIONS, ...overrides };
}

describe("Dynamic Actor Failed-Start Lifecycle", () => {
	describe("Startup coalescing and backoff", () => {
		test("normal request retries startup after backoff expires", async () => {
			vi.useRealTimers();
			const status = createDynamicRuntimeStatus();
			let callCount = 0;

			await expect(
				coalesceDynamicStartup(
					status,
					async () => {
						callCount++;
						throw new Error("startup failure");
					},
					opts({ retryInitialDelayMs: 10 }),
				),
			).rejects.toThrow();

			expect(status.state).toBe("failed_start");
			expect(callCount).toBe(1);

			// Set retryAt to the past so backoff is expired.
			status.retryAt = Date.now() - 1;

			await coalesceDynamicStartup(
				status,
				async () => {
					callCount++;
				},
				opts({ retryInitialDelayMs: 10 }),
			);

			expect(status.state).toBe("running");
			expect(callCount).toBe(2);
		});

		test("normal request during active backoff returns stored failed-start error", async () => {
			vi.useRealTimers();
			const status = createDynamicRuntimeStatus();

			await expect(
				coalesceDynamicStartup(
					status,
					async () => {
						throw new Error("broken loader");
					},
					opts({ retryInitialDelayMs: 60_000 }),
				),
			).rejects.toThrow();

			expect(status.state).toBe("failed_start");
			expect(status.retryAt!).toBeGreaterThan(Date.now());

			let startupCalled = false;
			await expect(
				coalesceDynamicStartup(
					status,
					async () => {
						startupCalled = true;
					},
					opts({ retryInitialDelayMs: 60_000 }),
				),
			).rejects.toThrow();

			expect(startupCalled).toBe(false);
			expect(status.state).toBe("failed_start");
		});

		test("no background retry loop runs while actor is in failed-start backoff", async () => {
			vi.useRealTimers();
			const status = createDynamicRuntimeStatus();
			let callCount = 0;

			await expect(
				coalesceDynamicStartup(
					status,
					async () => {
						callCount++;
						throw new Error("fail");
					},
					opts(),
				),
			).rejects.toThrow();

			expect(callCount).toBe(1);
			expect(status.state).toBe("failed_start");

			await new Promise((r) => setTimeout(r, 200));
			expect(callCount).toBe(1);
		});

		test("reload bypasses backoff and immediately retries startup", async () => {
			vi.useRealTimers();
			const status = createDynamicRuntimeStatus();

			await expect(
				coalesceDynamicStartup(
					status,
					async () => {
						throw new Error("fail");
					},
					opts({
						retryInitialDelayMs: 60_000,
						retryMaxDelayMs: 120_000,
					}),
				),
			).rejects.toThrow();

			expect(status.state).toBe("failed_start");
			expect(status.retryAt!).toBeGreaterThan(Date.now() + 30_000);

			transitionToInactive(status);

			expect(status.state).toBe("inactive");
			expect(status.retryAt).toBeUndefined();
			expect(status.retryAttempt).toBe(0);

			await coalesceDynamicStartup(
				status,
				async () => {},
				opts(),
			);

			expect(status.state).toBe("running");
		});

		test("reload on inactive actor is a no-op and does not cause double-load", async () => {
			vi.useRealTimers();
			const status = createDynamicRuntimeStatus();
			expect(status.state).toBe("inactive");

			const genBefore = status.generation;
			transitionToInactive(status);
			expect(status.state).toBe("inactive");
			expect(status.generation).toBe(genBefore);
		});

		test("concurrent requests coalesce onto one startup via shared startupPromise", async () => {
			vi.useRealTimers();
			const status = createDynamicRuntimeStatus();
			let callCount = 0;

			const startupFn = async () => {
				callCount++;
				await new Promise((r) => setTimeout(r, 50));
			};

			const p1 = coalesceDynamicStartup(status, startupFn, opts());
			const p2 = coalesceDynamicStartup(status, startupFn, opts());

			await Promise.all([p1, p2]);

			expect(callCount).toBe(1);
			expect(status.state).toBe("running");
		});

		test("stale startup generation cannot overwrite newer reload-triggered generation", async () => {
			vi.useRealTimers();
			const status = createDynamicRuntimeStatus();

			const slowStartup = coalesceDynamicStartup(
				status,
				async () => {
					await new Promise((r) => setTimeout(r, 100));
				},
				opts(),
			);

			expect(status.state).toBe("starting");
			const oldGeneration = status.generation;

			status.generation += 1;
			status.abortController?.abort();
			status.startupPromise?.reject(new Error("aborted by reload"));
			status.startupPromise = undefined;
			status.abortController = undefined;
			transitionToInactive(status);

			await slowStartup;

			expect(status.state).toBe("inactive");
			expect(status.generation).toBeGreaterThan(oldGeneration);
		});
	});

	describe("Error responses", () => {
		test("production response is sanitized (no details, has code)", async () => {
			vi.useRealTimers();
			const originalEnv = process.env.NODE_ENV;
			try {
				process.env.NODE_ENV = "production";
				const status = createDynamicRuntimeStatus();

				const error = await coalesceDynamicStartup(
					status,
					async () => {
						throw new Error("secret internal stack trace info");
					},
					opts(),
				).catch((e: unknown) => e);

				expect(error).toBeDefined();
				expect((error as any).message).toBe(SANITIZED_STARTUP_MESSAGE);
				expect((error as any).code).toBe("dynamic_startup_failed");
				expect((error as any).metadata?.details).toBeUndefined();
			} finally {
				process.env.NODE_ENV = originalEnv;
			}
		});

		test("development response includes full detail", async () => {
			vi.useRealTimers();
			const originalEnv = process.env.NODE_ENV;
			try {
				process.env.NODE_ENV = "test";
				const status = createDynamicRuntimeStatus();

				const error = await coalesceDynamicStartup(
					status,
					async () => {
						throw new Error("detailed error message");
					},
					opts(),
				).catch((e: unknown) => e);

				expect(error).toBeDefined();
				expect((error as any).message).toBe("detailed error message");
				expect((error as any).code).toBe("dynamic_startup_failed");
				expect((error as any).metadata?.details).toBeDefined();
			} finally {
				process.env.NODE_ENV = originalEnv;
			}
		});

		test("dynamic load timeout returns 'dynamic_load_timeout' error code", async () => {
			vi.useRealTimers();
			const status = createDynamicRuntimeStatus();

			const error = await coalesceDynamicStartup(
				status,
				async (signal) => {
					await new Promise((resolve, reject) => {
						signal.addEventListener("abort", () =>
							reject(signal.reason),
						);
					});
				},
				opts({ timeoutMs: 50 }),
			).catch((e: unknown) => e);

			expect(error).toBeDefined();
			expect((error as any).code).toBe("dynamic_load_timeout");
			expect(status.state).toBe("failed_start");
			expect(status.lastStartErrorCode).toBe("dynamic_load_timeout");
		});
	});

	describe("maxAttempts", () => {
		test("maxAttempts exhaustion tears down the wrapper", async () => {
			vi.useRealTimers();
			const status = createDynamicRuntimeStatus();
			const maxAttempts = 2;

			for (let i = 0; i < maxAttempts; i++) {
				if (status.retryAt) {
					status.retryAt = Date.now() - 1;
				}

				await expect(
					coalesceDynamicStartup(
						status,
						async () => {
							throw new Error(`fail ${i}`);
						},
						opts({ maxAttempts }),
					),
				).rejects.toThrow();
			}

			expect(status.state).toBe("inactive");
			expect(status.retryAttempt).toBe(0);
		});

		test("request after maxAttempts exhaustion triggers fresh startup from attempt 0", async () => {
			vi.useRealTimers();
			const status = createDynamicRuntimeStatus();
			const maxAttempts = 2;

			for (let i = 0; i < maxAttempts; i++) {
				if (status.retryAt) {
					status.retryAt = Date.now() - 1;
				}
				await expect(
					coalesceDynamicStartup(
						status,
						async () => {
							throw new Error(`fail ${i}`);
						},
						opts({ maxAttempts }),
					),
				).rejects.toThrow();
			}

			expect(status.state).toBe("inactive");

			await coalesceDynamicStartup(
				status,
				async () => {},
				opts({ maxAttempts }),
			);

			expect(status.state).toBe("running");
			expect(status.retryAttempt).toBe(0);
		});
	});

	describe("Reload-while-starting", () => {
		test("reload-while-starting aborts old attempt and starts new generation", async () => {
			vi.useRealTimers();
			const status = createDynamicRuntimeStatus();
			let firstAttemptAborted = false;

			const p1 = coalesceDynamicStartup(
				status,
				async (signal) => {
					await new Promise<void>((resolve, reject) => {
						signal.addEventListener("abort", () => {
							firstAttemptAborted = true;
							reject(signal.reason);
						});
					});
				},
				opts(),
			);

			expect(status.state).toBe("starting");
			const gen1 = status.generation;

			status.generation += 1;
			status.abortController?.abort(new Error("aborted by reload"));
			status.startupPromise?.reject(
				new Error("startup aborted by reload"),
			);
			status.startupPromise = undefined;
			status.abortController = undefined;
			transitionToInactive(status);

			await p1;

			expect(firstAttemptAborted).toBe(true);
			expect(status.generation).toBeGreaterThan(gen1);
			expect(status.state).toBe("inactive");

			await coalesceDynamicStartup(
				status,
				async () => {},
				opts(),
			);

			expect(status.state).toBe("running");
		});
	});
});
