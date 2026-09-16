import type { Sandbox, SandboxProcess } from "@rivet-dev/sandbox-adapter";
import { Effect, Sink, Stream } from "effect";
import { ChildProcess } from "effect/unstable/process";
import { describe, expect, it, vi } from "vitest";
import { sandboxEnvironment } from "../src/sandbox.js";

function fixture() {
	const started = Promise.withResolvers<void>();
	const process: SandboxProcess = {
		pid: 1,
		wait: () => new Promise(() => {}),
		writeStdin: vi.fn(async () => {}),
		closeStdin: vi.fn(async () => {}),
		kill: vi.fn(async () => {}),
		readOutput: vi.fn(async () => ({
			events: [],
			nextSequence: -1,
			hasMore: false,
			truncated: true,
		})),
	};
	const spawn = vi.fn(async () => {
		started.resolve();
		return process;
	});
	const sandbox = {
		cwd: "/workspace",
		binding: null,
		spawn,
	} as unknown as Sandbox;
	return {
		process,
		spawn,
		started: started.promise,
		environment: sandboxEnvironment(sandbox),
	};
}

describe("sandbox process lifecycle", () => {
	it("rejects unsupported stream options before spawning", async () => {
		const f = fixture();
		for (const options of [
			{ stdin: { stream: "pipe" as const, endOnDone: false } },
			{ stdout: Sink.drain.pipe(Sink.map(() => new Uint8Array())) },
		]) {
			await expect(
				Effect.runPromise(
					Effect.scoped(
						f.environment.spawner.spawn(ChildProcess.make("test", [], options)),
					),
				),
			).rejects.toThrow(/Sandbox/);
		}
		expect(f.spawn).not.toHaveBeenCalled();
	});
	it("rejects lost output and kills the scoped process", async () => {
		const f = fixture();
		await expect(
			Effect.runPromise(
				Effect.scoped(
					Effect.gen(function* () {
						const process = yield* f.environment.spawner.spawn(
							ChildProcess.make("test"),
						);
						return yield* Stream.runCollect(process.stdout);
					}),
				),
			),
		).rejects.toThrow("truncated");
		expect(f.process.kill).toHaveBeenCalledWith("SIGKILL");
	});

	it("cancels the remote process when its Effect scope is interrupted", async () => {
		const f = fixture();
		const controller = new AbortController();
		const running = Effect.runPromise(
			Effect.scoped(
				Effect.gen(function* () {
					yield* f.environment.spawner.spawn(ChildProcess.make("test"));
					yield* Effect.never;
				}),
			),
			{ signal: controller.signal },
		);
		const rejected = expect(running).rejects.toBeDefined();
		await f.started;
		controller.abort();
		await rejected;
		expect(f.process.kill).toHaveBeenCalledWith("SIGKILL");
	});
});
