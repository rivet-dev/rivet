import { spawn } from "node:child_process";
import { createRequire } from "node:module";
import { describe, expect, test } from "vitest";

const require = createRequire(import.meta.url);
const NAPI_BINDING = require.resolve("@rivetkit/rivetkit-napi");

interface NativeCancellationBindings {
	CancellationToken: new () => {
		cancel(): void;
		onCancelled(callback: () => void): { release(): void };
	};
}

function loadBindings(): NativeCancellationBindings {
	return require(NAPI_BINDING) as NativeCancellationBindings;
}

/** Waits for one turn of native callbacks to be delivered to JS. */
async function flushNativeCallbacks(): Promise<void> {
	const { CancellationToken } = loadBindings();
	const token = new CancellationToken();
	await new Promise<void>((resolve) => {
		token.onCancelled(resolve);
		token.cancel();
	});
}

describe("native cancellation listeners", () => {
	test("a cancelled token calls its listener once", async () => {
		const { CancellationToken } = loadBindings();
		const token = new CancellationToken();
		let calls = 0;
		const fired = new Promise<void>((resolve) => {
			token.onCancelled(() => {
				calls += 1;
				resolve();
			});
		});

		token.cancel();
		token.cancel();
		await fired;
		await flushNativeCallbacks();
		expect(calls).toBe(1);
	});

	test("a released listener is never called", async () => {
		const { CancellationToken } = loadBindings();
		const token = new CancellationToken();
		let calls = 0;
		const listener = token.onCancelled(() => {
			calls += 1;
		});

		listener.release();
		listener.release();
		token.cancel();
		await flushNativeCallbacks();
		expect(calls).toBe(0);
	});

	// Regression test for a per-request leak: every request registered a
	// listener whose native callback handle stayed alive until the token was
	// cancelled. Each live handle keeps the event loop referenced, so a process
	// that releases all of its listeners must still exit on its own.
	test("released listeners free their native callback handles", async () => {
		const script = `
			const { CancellationToken } = require(${JSON.stringify(NAPI_BINDING)});
			const tokens = [];
			for (let i = 0; i < 5000; i++) {
				const token = new CancellationToken();
				token.onCancelled(() => {}).release();
				tokens.push(token);
			}
		`;
		const child = spawn(process.execPath, ["-e", script], {
			stdio: "ignore",
		});
		const timeout = setTimeout(() => child.kill("SIGKILL"), 20_000);
		const exit = await new Promise<{
			code: number | null;
			signal: NodeJS.Signals | null;
		}>((resolve) => {
			child.on("exit", (code, signal) => resolve({ code, signal }));
		});
		clearTimeout(timeout);

		expect(exit).toEqual({ code: 0, signal: null });
	});
});
