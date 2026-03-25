import { Cause, Effect, Exit, Option } from "effect";
import { describe, expect, it, vi } from "@effect/vitest";
import type { AnyManagedRuntime } from "../src/runtime.ts";
import { runPromise, runPromiseExit, setManagedRuntime } from "../src/runtime.ts";

describe("@rivetkit/effect runtime", () => {
	it("runPromise executes successfully without actor runtime context", async () => {
		await expect(runPromise(Effect.succeed("ok"))).resolves.toBe("ok");
	});

	it("runPromiseExit converts runtime execution rejection into RuntimeExecutionError defect", async () => {
		const ctx = {};
		const runtime = {
			runPromiseExit: vi.fn(async () => {
				throw new Error("runtime crash");
			}),
		} as unknown as AnyManagedRuntime;

		setManagedRuntime(ctx, runtime);

		const exit = await runPromiseExit(Effect.succeed("ok"), ctx);
		expect(Exit.isFailure(exit)).toBe(true);

		if (Exit.isFailure(exit)) {
			const defect = Cause.dieOption(exit.cause);
			expect(Option.isSome(defect)).toBe(true);
			if (Option.isSome(defect)) {
				expect((defect.value as any)?._tag).toBe("RuntimeExecutionError");
				expect((defect.value as any)?.operation).toBe("runPromiseExit");
			}
		}
	});

	it("runPromise rejects with RuntimeExecutionError when runtime execution fails", async () => {
		const ctx = {};
		const runtime = {
			runPromiseExit: vi.fn(async () => {
				throw new Error("runtime crash");
			}),
		} as unknown as AnyManagedRuntime;

		setManagedRuntime(ctx, runtime);

		await expect(runPromise(Effect.succeed("ok"), ctx)).rejects.toMatchObject({
			_tag: "RuntimeExecutionError",
			operation: "runPromiseExit",
		});
	});
});
