import { execFile as execFileCallback, spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { promisify } from "node:util";
import type {
	Sandbox,
	SandboxOutputEvent,
	SandboxProcessExit,
} from "@rivet-dev/sandbox-adapter";
import { Effect, Stream } from "effect";
import { ChildProcess } from "effect/unstable/process";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { sandboxEnvironment } from "../src/sandbox.js";
import { opencode } from "../dist/index.js";
import { sqliteFixture } from "./sqlite-fixture.js";

const execFile = promisify(execFileCallback);
const container = `rivet-opencode-test-${randomUUID()}`;
const docker = (...args: string[]) => execFile("docker", args);

function sandbox(): Sandbox {
	// Only spawn is needed: OpenCode derives its file driver from processes.
	return {
		binding: { container },
		cwd: "/workspace",
		async spawn(command, args = [], options) {
			const process = spawn(
				"docker",
				[
					"exec",
					"-i",
					"--workdir",
					options?.cwd ?? "/workspace",
					...Object.entries(options?.env ?? {}).flatMap(([key, value]) => [
						"--env",
						`${key}=${value}`,
					]),
					container,
					command,
					...args,
				],
				{ stdio: "pipe" },
			);
			const events: SandboxOutputEvent[] = [];
			process.stdout.on("data", (data) =>
				events.push({ sequence: events.length, stream: "stdout", data }),
			);
			process.stderr.on("data", (data) =>
				events.push({ sequence: events.length, stream: "stderr", data }),
			);
			const done = new Promise<SandboxProcessExit>((resolve, reject) => {
				process.on("error", reject);
				process.on("close", (exitCode, signal) =>
					resolve({
						exitCode,
						outcome: signal ? "signalled" : "exited",
						signal: signal ?? undefined,
					}),
				);
			});
			return {
				pid: process.pid!,
				wait: () => done,
				async writeStdin(bytes) {
					await new Promise<void>((resolve, reject) =>
						process.stdin.write(bytes, (error) =>
							error ? reject(error) : resolve(),
						),
					);
				},
				async closeStdin() {
					process.stdin.end();
				},
				async kill(signal) {
					process.kill(signal as NodeJS.Signals);
				},
				async readOutput({ after = -1, maxEvents = 64 } = {}) {
					const page = events
						.filter((event) => event.sequence > after)
						.slice(0, maxEvents);
					const nextSequence = page.at(-1)?.sequence ?? after;
					return {
						events: page,
						nextSequence,
						hasMore: nextSequence < events.length - 1,
						truncated: false,
					};
				},
			};
		},
	} satisfies Pick<Sandbox, "binding" | "cwd" | "spawn"> as unknown as Sandbox;
}

describe.runIf(process.env.OPENCODE_DOCKER_TESTS === "1")(
	"Docker sandbox",
	() => {
		beforeAll(async () => {
			await docker(
				"run",
				"--detach",
				"--rm",
				"--name",
				container,
				"node:22-bookworm-slim",
				"sleep",
				"300",
			);
			await docker("exec", container, "mkdir", "-p", "/workspace");
		}, 120_000);
		afterAll(async () => {
			await docker("rm", "--force", container);
		});

		it("streams stdin, files, pipelines, and large output in the sandbox", async () => {
			const env = sandboxEnvironment(sandbox());
			await Effect.runPromise(
				env.files.write(
					"/workspace/hello.txt",
					new TextEncoder().encode("hello sandbox"),
				),
			);
			const file = await Effect.runPromise(
				env.files.read("/workspace/hello.txt"),
			);
			expect(new TextDecoder().decode(file.bytes)).toBe("hello sandbox");
			expect(await Effect.runPromise(env.files.list("/workspace"))).toEqual(
				expect.arrayContaining([
					expect.objectContaining({ name: "hello.txt" }),
				]),
			);
			const pipeline = ChildProcess.make("printf", ["pipe works"]).pipe(
				ChildProcess.pipeTo(ChildProcess.make("cat")),
			);
			expect(await Effect.runPromise(env.spawner.string(pipeline))).toBe(
				"pipe works",
			);
			expect(
				await Effect.runPromise(
					env.spawner.string(
						ChildProcess.make("printf shell && printf ' works'", [], {
							shell: true,
						}),
					),
				),
			).toBe("shell works");
			const large = ChildProcess.make("head", ["-c", "2097152", "/dev/zero"]);
			const chunks = await Effect.runPromise(
				Effect.scoped(
					Effect.gen(function* () {
						const process = yield* env.spawner.spawn(large);
						return yield* Stream.runCollect(process.stdout);
					}),
				),
			);
			expect(chunks.reduce((sum, chunk) => sum + chunk.length, 0)).toBe(
				2097152,
			);
		}, 30_000);

		it("runs the actual SDK with a remote-only cwd and strips host environment", async () => {
			const { db } = sqliteFixture();
			process.env.RIVET_OPENCODE_TEST_SECRET = "host-only-value";
			const mounted = sandbox();
			const definition = opencode({
				sandbox: { connect: async () => mounted },
				opencode: { models: { fetch: false } },
			});
			const config = definition.config as any;
			const context: any = {
				actorId: "docker-test",
				key: [],
				db,
				client: () => ({}),
				keepAwake: (p: Promise<unknown>) => p,
				broadcast: () => {},
				log: { error: () => {} },
			};
			await db.execute(
				"CREATE TABLE _rivet_opencode (id INTEGER PRIMARY KEY, binding TEXT, cwd TEXT NOT NULL)",
			);
			try {
				await config.onWake(context);
				const session = await config.actions.getSession(context);
				await config.actions.session.shell(context, {
					sessionID: session.id,
					command:
						"printf '%s' \"${RIVET_OPENCODE_TEST_SECRET-unset}\" > /workspace/env.txt",
				});
				await config.actions.waitForIdle(context);
				expect(
					(await docker("exec", container, "cat", "/workspace/env.txt")).stdout,
				).toBe("unset");
			} finally {
				delete process.env.RIVET_OPENCODE_TEST_SECRET;
				await config.onDestroy(context);
				await db.close();
			}
		}, 60_000);
	},
);
