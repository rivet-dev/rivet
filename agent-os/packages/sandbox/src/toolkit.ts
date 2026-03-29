/**
 * Sandbox toolkit exposing process management and command execution
 * as host tools for agents running inside an agentOS VM.
 */

import type { SandboxAgent } from "sandbox-agent";
import type { HostTool, ToolKit } from "@rivet-dev/agent-os-core";
import { z } from "zod";

export interface SandboxToolkitOptions {
	/** A connected SandboxAgent client instance. */
	client: SandboxAgent;
}

/** Host tool type alias for convenience. */
function hostTool<INPUT, OUTPUT>(
	def: HostTool<INPUT, OUTPUT>,
): HostTool<INPUT, OUTPUT> {
	return def;
}

/**
 * Create a ToolKit that exposes sandbox process management operations.
 */
export function createSandboxToolkit(options: SandboxToolkitOptions): ToolKit {
	const { client } = options;

	return {
		name: "sandbox",
		description: "Execute commands and manage processes in a remote sandbox environment.",
		tools: {
			"run-command": hostTool({
				description:
					"Run a command synchronously in the sandbox and return its stdout, stderr, and exit code.",
				inputSchema: z.object({
					command: z.string().describe("The command to execute (e.g. 'ls', 'python3')."),
					args: z
						.array(z.string())
						.optional()
						.describe("Arguments to pass to the command."),
					cwd: z
						.string()
						.optional()
						.describe("Working directory for the command."),
					env: z
						.record(z.string())
						.optional()
						.describe("Additional environment variables."),
					timeoutMs: z
						.number()
						.optional()
						.describe("Maximum execution time in milliseconds."),
				}),
				timeout: 120_000,
				execute: async (input) => {
					const result = await client.runProcess({
						command: input.command,
						args: input.args,
						cwd: input.cwd,
						env: input.env,
						timeoutMs: input.timeoutMs,
					});
					return {
						stdout: result.stdout,
						stderr: result.stderr,
						exitCode: result.exitCode,
						timedOut: result.timedOut,
						durationMs: result.durationMs,
					};
				},
			}),

			"create-process": hostTool({
				description:
					"Start a long-running background process in the sandbox. Returns a process ID for later management.",
				inputSchema: z.object({
					command: z.string().describe("The command to execute."),
					args: z
						.array(z.string())
						.optional()
						.describe("Arguments to pass to the command."),
					cwd: z
						.string()
						.optional()
						.describe("Working directory for the process."),
					env: z
						.record(z.string())
						.optional()
						.describe("Additional environment variables."),
				}),
				execute: async (input) => {
					const proc = await client.createProcess({
						command: input.command,
						args: input.args,
						cwd: input.cwd,
						env: input.env,
					});
					return {
						id: proc.id,
						command: proc.command,
						args: proc.args,
						status: proc.status,
						pid: proc.pid,
					};
				},
			}),

			"list-processes": hostTool({
				description: "List all processes running in the sandbox.",
				inputSchema: z.object({}),
				execute: async () => {
					const result = await client.listProcesses();
					return {
						processes: result.processes.map((p) => ({
							id: p.id,
							command: p.command,
							args: p.args,
							status: p.status,
							exitCode: p.exitCode,
							pid: p.pid,
						})),
					};
				},
			}),

			"stop-process": hostTool({
				description: "Gracefully stop a running process in the sandbox.",
				inputSchema: z.object({
					id: z.string().describe("The process ID to stop."),
				}),
				execute: async (input) => {
					const proc = await client.stopProcess(input.id);
					return {
						id: proc.id,
						status: proc.status,
						exitCode: proc.exitCode,
					};
				},
			}),

			"kill-process": hostTool({
				description: "Forcefully kill a running process in the sandbox.",
				inputSchema: z.object({
					id: z.string().describe("The process ID to kill."),
				}),
				execute: async (input) => {
					const proc = await client.killProcess(input.id);
					return {
						id: proc.id,
						status: proc.status,
						exitCode: proc.exitCode,
					};
				},
			}),

			"get-process-logs": hostTool({
				description: "Get stdout/stderr logs from a sandbox process.",
				inputSchema: z.object({
					id: z.string().describe("The process ID."),
					stream: z
						.enum(["stdout", "stderr", "combined"])
						.optional()
						.describe("Which output stream to read. Defaults to combined."),
					tail: z
						.number()
						.optional()
						.describe("Only return the last N log entries."),
				}),
				execute: async (input) => {
					const result = await client.getProcessLogs(input.id, {
						stream: input.stream as "stdout" | "stderr" | "combined" | undefined,
						tail: input.tail,
					});
					return {
						logs: result.entries.map((e) => {
							// The sandbox-agent SDK returns log data as base64. Decode it
							// so callers receive plain text.
							let text = e.data;
							if (e.encoding === "base64") {
								text = Buffer.from(e.data, "base64").toString("utf-8");
							}
							return {
								data: text,
								stream: e.stream,
								timestampMs: e.timestampMs,
							};
						}),
					};
				},
			}),

			"send-input": hostTool({
				description: "Send text input to an interactive sandbox process via stdin.",
				inputSchema: z.object({
					id: z.string().describe("The process ID."),
					data: z.string().describe("The text to send to the process stdin."),
				}),
				execute: async (input) => {
					// Encode the text as base64 for the sandbox-agent API.
					const encoded = Buffer.from(input.data, "utf-8").toString("base64");
					await client.sendProcessInput(input.id, {
						data: encoded,
						encoding: "base64",
					});
					return { sent: true };
				},
			}),
		},
	};
}
