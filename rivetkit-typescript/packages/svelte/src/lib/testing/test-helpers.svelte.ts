/**
 * Svelte 5 rune-aware test helpers.
 *
 * Svelte 5 runes (`$state`, `$effect`) require a reactive root context
 * to execute. These helpers create that context inside vitest (or any
 * test runner), following the pattern established by runed.
 *
 * @module
 */

import { test } from "vitest";

/**
 * Run a test inside a Svelte 5 `$effect.root` so that `$state`,
 * `$derived`, and `$effect` work correctly.
 *
 * @param name - Test name (passed to vitest `test()`).
 * @param fn - Test body. May be async. `$state` and `$effect` are available inside.
 *
 * @example
 * ```typescript
 * import { describe, expect } from "vitest";
 * import { testWithEffect } from "@rivetkit/svelte/testing";
 * import { flushSync } from "svelte";
 *
 * describe("useActor", () => {
 *   testWithEffect("returns idle status initially", () => {
 *     let status = $state("idle");
 *     expect(status).toBe("idle");
 *   });
 * });
 * ```
 */
export function testWithEffect(
	name: string,
	fn: () => void | Promise<void>,
): void {
	test(name, () => effectRootScope(fn));
}

/**
 * Execute a function inside a Svelte 5 `$effect.root`.
 *
 * Useful when you need `$effect.root` without the vitest `test()` wrapper
 * (e.g. inside `beforeEach` or custom setup functions).
 *
 * @param fn - Function to execute. May be async.
 * @returns `void` or a `Promise<void>` that resolves when the function completes.
 */
export function effectRootScope(
	fn: () => void | Promise<void>,
): void | Promise<void> {
	let promise!: void | Promise<void>;
	const cleanup = $effect.root(() => {
		promise = fn();
	});
	if (promise instanceof Promise) {
		return promise.finally(cleanup);
	}
	cleanup();
}
