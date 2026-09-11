import { env as hostEnvironment } from "node:process";
import { Environment } from "@opencode/core/environment/index";
import type { Sandbox, SandboxProcess } from "@rivet-dev/sandbox-adapter";
import { Effect, PlatformError, Sink, Stream } from "effect";
import * as Spawner from "effect/unstable/process/ChildProcessSpawner";

const failure = (cause: unknown) =>
	PlatformError.systemError({
		_tag: "Unknown",
		module: "Sandbox",
		method: "spawn",
		cause,
		description: cause instanceof Error ? cause.message : String(cause),
	});
const attempt = <T>(run: () => Promise<T>) =>
	Effect.tryPromise({ try: run, catch: failure });

/** OpenCode's environment seam supplies all coding tools, including file operations. */
export function sandboxEnvironment(sandbox: Sandbox): Environment.Interface {
	const spawn: Spawner.ChildProcessSpawner["Service"]["spawn"] = (command) =>
		Effect.gen(function* () {
			if (command._tag === "PipedCommand") {
				if (command.options.to && command.options.to !== "stdin") {
					return yield* Effect.fail(
						failure(new Error("Sandbox pipelines only support stdin")),
					);
				}
				const left = yield* spawn(command.left);
				const right = yield* spawn(command.right);
				const from = command.options.from ?? "stdout";
				if (from !== "stdout" && from !== "stderr" && from !== "all") {
					return yield* Effect.fail(
						failure(
							new Error(
								"Sandbox pipelines only support standard output streams",
							),
						),
					);
				}
				yield* Stream.run(left[from], right.stdin).pipe(Effect.forkScoped);
				return Spawner.makeHandle({ ...right, stdin: left.stdin });
			}
			if (
				command.options.additionalFds &&
				Object.keys(command.options.additionalFds).length
			) {
				return yield* Effect.fail(
					failure(
						new Error("Sandbox additional file descriptors are unsupported"),
					),
				);
			}
			const inputConfig = command.options.stdin;
			if (
				inputConfig &&
				typeof inputConfig === "object" &&
				!Stream.isStream(inputConfig) &&
				inputConfig.endOnDone === false
			) {
				return yield* Effect.fail(
					failure(
						new Error(
							"Sandbox stdin must close when its input stream completes",
						),
					),
				);
			}
			for (const configured of [
				command.options.stdout,
				command.options.stderr,
			]) {
				const output =
					configured &&
					typeof configured === "object" &&
					!Sink.isSink(configured)
						? configured.stream
						: configured;
				if (Sink.isSink(output))
					return yield* Effect.fail(
						failure(
							new Error(
								"Sandbox output sinks are unsupported; consume the process output streams",
							),
						),
					);
			}
			const controller = new AbortController();
			const process = yield* Effect.acquireRelease(
				attempt(() =>
					sandbox.spawn(
						command.options.shell
							? typeof command.options.shell === "string"
								? command.options.shell
								: "sh"
							: command.command,
						command.options.shell
							? ["-c", [command.command, ...command.args.map(quote)].join(" ")]
							: command.args,
						{
							cwd: command.options.cwd ?? sandbox.cwd,
							// OpenCode sometimes explicitly spreads process.env. Strip inherited
							// values too; sandbox credentials belong in the provider configuration.
							env: Object.fromEntries(
								Object.entries(command.options.env ?? {}).filter(
									(entry): entry is [string, string] =>
										entry[1] !== undefined &&
										entry[1] !== hostEnvironment[entry[0]],
								),
							),
							signal: controller.signal,
							retainOutput: true,
						},
					),
				),
				(process) =>
					Effect.promise(async () => {
						controller.abort();
						await process.kill("SIGKILL");
					}).pipe(Effect.catchCause(() => Effect.void)),
			);
			let exited = false;
			const exit = process.wait().then(
				(result) => {
					exited = true;
					return result;
				},
				(error) => {
					exited = true;
					throw error;
				},
			);
			// Attach a rejection handler even when the consumer only reads output.
			void exit.catch(() => {});
			const output = (stream?: "stdout" | "stderr") =>
				processOutput(process, () => exited, stream);
			const stdin = Sink.forEach((bytes: Uint8Array) =>
				attempt(() => process.writeStdin(bytes)),
			).pipe(
				Sink.ensuring(attempt(() => process.closeStdin()).pipe(Effect.orDie)),
			);
			const handle = Spawner.makeHandle({
				pid: Spawner.ProcessId(
					typeof process.pid === "number" ? process.pid : 0,
				),
				exitCode: attempt(async () => {
					const result = await exit;
					return Spawner.ExitCode(
						result.exitCode ?? (result.outcome === "timed_out" ? 124 : 128),
					);
				}),
				isRunning: Effect.sync(() => !exited),
				kill: (options) =>
					attempt(() => process.kill(options?.killSignal ?? "SIGTERM")),
				stdin,
				stdout: output("stdout"),
				stderr: output("stderr"),
				all: output(),
				getInputFd: () =>
					Sink.fail(
						failure(
							new Error("Sandbox additional file descriptors are unsupported"),
						),
					),
				getOutputFd: () =>
					Stream.fail(
						failure(
							new Error("Sandbox additional file descriptors are unsupported"),
						),
					),
				unref: Effect.succeed(Effect.void),
			});
			const configuredInput = command.options.stdin;
			const input =
				configuredInput &&
				typeof configuredInput === "object" &&
				!Stream.isStream(configuredInput)
					? configuredInput.stream
					: configuredInput;
			if (Stream.isStream(input))
				yield* Stream.run(input, stdin).pipe(Effect.forkScoped);
			else if (input === "ignore" || input === "inherit")
				yield* attempt(() => process.closeStdin());
			return handle;
		});
	const spawner = Spawner.make(spawn);
	return { spawner, files: Environment.makeFiles({ spawner }) };
}

function processOutput(
	process: SandboxProcess,
	exited: () => boolean,
	stream?: "stdout" | "stderr",
) {
	return Stream.unfold(-1, (after) =>
		Effect.gen(function* () {
			const done = exited();
			const page = yield* attempt(() =>
				process.readOutput({ after, maxEvents: 64 }),
			);
			if (page.truncated)
				return yield* Effect.fail(
					failure(new Error("Sandbox process output was truncated")),
				);
			const events = page.events.filter(
				(event) =>
					!stream ||
					event.stream === stream ||
					(stream === "stdout" && event.stream === "pty"),
			);
			if (done && page.events.length === 0 && !page.hasMore) return undefined;
			// The shared adapter exposes pull cursors, so idle reads need a bounded delay.
			if (page.events.length === 0 && !page.hasMore) yield* Effect.sleep(25);
			return [events.map((event) => event.data), page.nextSequence] as const;
		}),
	).pipe(Stream.flatMap((chunks) => Stream.fromIterable(chunks)));
}

function quote(value: string): string {
	return `'${value.replaceAll("'", `'"'"'`)}'`;
}
