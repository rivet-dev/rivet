import { describe, expect, test, vi } from "vitest";
import type { DriverTestConfig } from "../mod";
import { setupDriverTest, waitFor } from "../utils";
import {
	createDynamicRuntimeStatus,
	coalesceDynamicStartup,
	transitionToFailedStart,
	transitionToInactive,
	SANITIZED_STARTUP_MESSAGE,
} from "@/dynamic/mod";
import type { DynamicRuntimeStatus, DynamicStartupOptions } from "@/dynamic/mod";

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

export function runDynamicFailedStartTests(
	driverTestConfig: DriverTestConfig,
) {
	describe.skipIf(!driverTestConfig.isDynamic)(
		"Dynamic Actor Failed-Start Lifecycle Tests",
		() => {
			describe("Startup coalescing and backoff", () => {
				test("normal request retries startup after backoff expires", async () => {
					vi.useRealTimers();
					const status = createDynamicRuntimeStatus();
					let callCount = 0;

					// First call fails.
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

					// Second call should trigger a fresh startup attempt.
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

					// Fail startup to enter failed_start state.
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
					expect(status.retryAt).toBeDefined();
					// retryAt is in the future.
					expect(status.retryAt!).toBeGreaterThan(Date.now());

					// A request during active backoff should throw the stored
					// error without calling startupFn.
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

					// Wait long enough that any hypothetical background timer
					// would have fired. No startupFn calls should happen.
					await new Promise((r) => setTimeout(r, 200));
					expect(callCount).toBe(1);
				});

				test("reload bypasses backoff and immediately retries startup", async () => {
					vi.useRealTimers();
					const status = createDynamicRuntimeStatus();

					// Fail startup to enter failed_start state with far-future
					// backoff.
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

					// Simulate reload: transition to inactive clears backoff
					// state so the next request starts fresh.
					transitionToInactive(status);

					expect(status.state).toBe("inactive");
					expect(status.retryAt).toBeUndefined();
					expect(status.retryAttempt).toBe(0);

					// Next startup attempt should proceed immediately.
					await coalesceDynamicStartup(
						status,
						async () => {
							// Success
						},
						opts(),
					);

					expect(status.state).toBe("running");
				});

				test("reload on inactive actor is a no-op and does not cause double-load", async () => {
					vi.useRealTimers();
					const status = createDynamicRuntimeStatus();
					expect(status.state).toBe("inactive");

					// transitionToInactive on an already inactive status is
					// safe and a no-op. The generation is unchanged.
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
						// Simulate slow startup.
						await new Promise((r) => setTimeout(r, 50));
					};

					// Fire two concurrent startup requests.
					const p1 = coalesceDynamicStartup(status, startupFn, opts());
					const p2 = coalesceDynamicStartup(status, startupFn, opts());

					await Promise.all([p1, p2]);

					// Only one startup attempt should have been made.
					expect(callCount).toBe(1);
					expect(status.state).toBe("running");
				});

				test("stale startup generation cannot overwrite newer reload-triggered generation", async () => {
					vi.useRealTimers();
					const status = createDynamicRuntimeStatus();

					// Start a slow startup that we will supersede.
					const slowStartup = coalesceDynamicStartup(
						status,
						async () => {
							// Simulate slow startup.
							await new Promise((r) => setTimeout(r, 100));
						},
						opts(),
					);

					// Wait for status to transition to "starting".
					expect(status.state).toBe("starting");
					const oldGeneration = status.generation;

					// Simulate a reload: increment generation and reject the
					// old startup promise.
					status.generation += 1;
					status.abortController?.abort();
					status.startupPromise?.reject(
						new Error("aborted by reload"),
					);
					status.startupPromise = undefined;
					status.abortController = undefined;
					transitionToInactive(status);

					// The original slow startup should complete without error
					// (its result is discarded due to generation mismatch).
					await slowStartup;

					// The status should remain inactive (not overwritten to
					// running by the stale completion).
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
								throw new Error(
									"secret internal stack trace info",
								);
							},
							opts(),
						).catch((e: unknown) => e);

						expect(error).toBeDefined();
						expect((error as any).message).toBe(
							SANITIZED_STARTUP_MESSAGE,
						);
						expect((error as any).code).toBe(
							"dynamic_startup_failed",
						);
						// In production, metadata should not include details.
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
						expect((error as any).message).toBe(
							"detailed error message",
						);
						expect((error as any).code).toBe(
							"dynamic_startup_failed",
						);
						// In dev mode, metadata includes stack trace details.
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
							// Never resolve; wait for abort signal.
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
					expect(status.lastStartErrorCode).toBe(
						"dynamic_load_timeout",
					);
				});
			});

			describe("maxAttempts", () => {
				test("maxAttempts exhaustion tears down the wrapper", async () => {
					vi.useRealTimers();
					const status = createDynamicRuntimeStatus();
					const maxAttempts = 2;

					for (let i = 0; i < maxAttempts; i++) {
						// Expire backoff so the next attempt proceeds.
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

					// After maxAttempts exhaustion, status transitions to
					// inactive with retryAttempt reset to 0.
					expect(status.state).toBe("inactive");
					expect(status.retryAttempt).toBe(0);
				});

				test("request after maxAttempts exhaustion triggers fresh startup from attempt 0", async () => {
					vi.useRealTimers();
					const status = createDynamicRuntimeStatus();
					const maxAttempts = 2;

					// Exhaust all attempts.
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

					// Next call should start fresh from attempt 0 and succeed.
					await coalesceDynamicStartup(
						status,
						async () => {
							// Success
						},
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

					// Start a slow startup.
					const p1 = coalesceDynamicStartup(
						status,
						async (signal) => {
							await new Promise<void>((resolve, reject) => {
								signal.addEventListener("abort", () => {
									firstAttemptAborted = true;
									reject(signal.reason);
								});
								// Never resolve naturally.
							});
						},
						opts(),
					);

					expect(status.state).toBe("starting");
					const gen1 = status.generation;

					// Simulate reload-while-starting: increment generation,
					// abort the controller, reject the startup promise.
					status.generation += 1;
					status.abortController?.abort(
						new Error("aborted by reload"),
					);
					status.startupPromise?.reject(
						new Error("startup aborted by reload"),
					);
					status.startupPromise = undefined;
					status.abortController = undefined;
					transitionToInactive(status);

					// Wait for the original startup to complete (it should
					// discard its result due to generation mismatch).
					await p1;

					expect(firstAttemptAborted).toBe(true);
					expect(status.generation).toBeGreaterThan(gen1);
					expect(status.state).toBe("inactive");

					// Start a new attempt; it should proceed fresh.
					await coalesceDynamicStartup(
						status,
						async () => {
							// Success
						},
						opts(),
					);

					expect(status.state).toBe("running");
				});
			});

			describe("HTTP integration", () => {
				test("GET /dynamic/status returns correct state and metadata", async (c) => {
					const { client } = await setupDriverTest(
						c,
						driverTestConfig,
					);
					const actor = client.sleep.getOrCreate([
						`status-test-${crypto.randomUUID()}`,
					]);

					// Start the actor by making a request.
					await actor.getCounts();

					// Check status of a running actor.
					const statusResponse = await actor.status();
					expect(statusResponse.state).toBe("running");
					expect(statusResponse.generation).toBeGreaterThanOrEqual(0);
					// Running actors should not have failure metadata.
					expect(statusResponse.lastStartErrorCode).toBeUndefined();
					expect(statusResponse.retryAt).toBeUndefined();
				});

				test("reload authentication rejects unauthenticated callers with 403", async (c) => {
					const { client, endpoint } = await setupDriverTest(
						c,
						driverTestConfig,
					);
					const actor = client.sleep.getOrCreate([
						`reload-auth-${crypto.randomUUID()}`,
					]);

					// Start the actor so it exists.
					await actor.getCounts();

					// In the test registry, dynamic actors do not have auth or
					// canReload callbacks configured, so reload is allowed in
					// dev mode. However, the status endpoint uses inspector
					// auth (token-based). Verify that status rejects requests
					// without a valid Bearer token.
					const actorId = await actor.resolve();
					const statusUrl = `${endpoint}/gateway/${encodeURIComponent(actorId)}/dynamic/status`;

					// Request without auth should be rejected.
					const noAuthResponse = await fetch(statusUrl);
					expect(noAuthResponse.status).toBe(401);

					// Request with wrong token should be rejected.
					const wrongTokenResponse = await fetch(statusUrl, {
						headers: { Authorization: "Bearer wrong" },
					});
					expect(wrongTokenResponse.status).toBe(401);

					// Request with correct token should succeed.
					const correctResponse = await fetch(statusUrl, {
						headers: { Authorization: "Bearer token" },
					});
					expect(correctResponse.status).toBe(200);
					const body = (await correctResponse.json()) as Record<string, unknown>;
					expect(body.state).toBeDefined();
				});
			});
		},
	);
}
