import { describe, expect, it, vi } from "vitest";
import { agentOSSandbox } from "../src/index.js";

function fixture() {
	const outputEvents = [
		{
			sequence: 0,
			channel: "stdout" as const,
			chunk: { encoding: "base64" as const, data: "b2s=" },
		},
	];
	const handle = {
		process: {
			spawn: vi.fn(async () => ({ pid: 42 })),
			wait: vi.fn(async () => ({
				outcome: "exited" as const,
				exitCode: 0,
			})),
			kill: vi.fn(async () => {}),
			signal: vi.fn(async () => {}),
			writeStdin: vi.fn(async () => {}),
			closeStdin: vi.fn(async () => {}),
			readOutput: vi.fn(async (_pid: number, options?: { after?: number }) => ({
				events: outputEvents.filter(
					(event) => event.sequence > (options?.after ?? -1),
				),
				nextCursor: "0",
				hasMore: false,
				truncated: false,
			})),
		},
		filesystem: {
			readFile: vi.fn(async () => new Uint8Array([1, 2, 3])),
			writeFile: vi.fn(async () => {}),
			stat: vi.fn(async () => ({ type: "file" as const, size: 3 })),
			readdir: vi.fn(async () => ["file.txt"]),
			exists: vi.fn(async () => true),
			mkdir: vi.fn(async () => {}),
			remove: vi.fn(async () => {}),
		},
		destroy: vi.fn(async () => {}),
	};
	const getOrCreate = vi.fn(() => handle);
	const context = {
		actorId: "actor-1",
		key: ["agent"],
		client: () => ({ sandboxes: { getOrCreate } }),
	};
	return { context, getOrCreate, handle };
}

describe("agentOSSandbox", () => {
	it("uses a deterministic actor key and reconnects from its binding", async () => {
		const { context, getOrCreate } = fixture();
		const adapter = agentOSSandbox({ actor: "sandboxes" });
		const first = await adapter.connect(context, { id: "primary" });
		expect(getOrCreate).toHaveBeenCalledWith(
			["sandbox", "actor-1", "primary"],
			{ params: undefined },
		);

		await adapter.connect(context, {
			id: "primary",
			binding: first.binding,
		});
		expect(getOrCreate).toHaveBeenLastCalledWith(
			["sandbox", "actor-1", "primary"],
			{ params: undefined },
		);
	});

	it("normalizes commands, process output, and destruction", async () => {
		const { context, handle } = fixture();
		const adapter = agentOSSandbox({ actor: "sandboxes" });
		const sandbox = await adapter.connect(context, { id: "primary" });
		const output: string[] = [];
		expect(
			await sandbox.exec("pwd", {
				onOutput: (event) => output.push(Buffer.from(event.data).toString()),
			}),
		).toMatchObject({ exitCode: 0, stdout: "ok", outcome: "exited" });
		expect(output).toEqual(["ok"]);

		const process = await sandbox.spawn("node", ["script.js"]);
		const page = await process.readOutput({ after: -1 });
		expect(page.nextSequence).toBe(0);
		expect(Buffer.from(page.events[0]!.data).toString()).toBe("ok");

		await adapter.destroy?.(context, {
			id: "primary",
			binding: sandbox.binding,
			sandbox,
		});
		expect(handle.destroy).toHaveBeenCalledOnce();
	});

	it("kills a process when cancellation races with spawn admission", async () => {
		const { context, handle } = fixture();
		let admit: (() => void) | undefined;
		handle.process.spawn.mockImplementationOnce(
			() =>
				new Promise<{ pid: number }>((resolve) => {
					admit = () => resolve({ pid: 42 });
				}),
		);
		const sandbox = await agentOSSandbox({ actor: "sandboxes" }).connect(
			context,
			{ id: "primary" },
		);
		const controller = new AbortController();
		const spawning = sandbox.spawn("sleep", ["60"], {
			signal: controller.signal,
		});
		controller.abort(new Error("cancelled"));
		admit?.();
		await expect(spawning).rejects.toThrow("cancelled");
		expect(handle.process.signal).toHaveBeenCalledWith(42, "SIGTERM");
	});

	it("bounds buffered exec output at the adapter", async () => {
		const { context, handle } = fixture();
		const sandbox = await agentOSSandbox({ actor: "sandboxes" }).connect(
			context,
			{ id: "primary" },
		);
		await expect(
			sandbox.exec("printf ok", { maxOutputBytes: 1 }),
		).resolves.toMatchObject({ stdout: "o", truncated: true });
		expect(handle.process.signal).toHaveBeenCalledWith(42, "SIGTERM");
	});

	it("cancels an admitted exec process", async () => {
		const { context, handle } = fixture();
		let finishWait: (() => void) | undefined;
		handle.process.wait.mockImplementationOnce(
			() =>
				new Promise((resolve) => {
					finishWait = () =>
						resolve({ outcome: "exited" as const, exitCode: 143 });
				}),
		);
		handle.process.signal.mockImplementationOnce(async () => finishWait?.());
		const sandbox = await agentOSSandbox({ actor: "sandboxes" }).connect(
			context,
			{ id: "primary" },
		);
		const controller = new AbortController();
		const execution = sandbox.exec("sleep 60", { signal: controller.signal });
		await vi.waitFor(() => expect(finishWait).toBeTypeOf("function"));
		controller.abort(new Error("cancelled"));
		await expect(execution).rejects.toThrow("cancelled");
		expect(handle.process.signal).toHaveBeenCalledWith(42, "SIGTERM");
	});

	it("fails closed when the selected actor is not agentOS", async () => {
		const adapter = agentOSSandbox({ actor: "wrong" });
		await expect(
			adapter.connect(
				{ actorId: "actor-1", key: [], client: () => ({}) },
				{ id: "primary" },
			),
		).rejects.toMatchObject({ code: "agentos_sandbox_configuration" });
	});
});
