import type {
	ProcessInfo,
	ProcessTreeNode,
	SpawnedProcessInfo,
} from "@rivet-dev/agent-os-core";
import { ActorStopping } from "@/actor/errors";
import type { AgentOsActorConfig } from "../config";
import type { AgentOsActionContext } from "../types";
import { ensureVm, syncPreventSleep } from "./index";

// Infer types from AgentOs methods since @secure-exec/core is not a direct dep.
type ExecResult = Awaited<
	ReturnType<import("@rivet-dev/agent-os-core").AgentOs["exec"]>
>;
type ExecOptions = Parameters<import("@rivet-dev/agent-os-core").AgentOs["exec"]>[1];
type SpawnOptions = Parameters<
	import("@rivet-dev/agent-os-core").AgentOs["spawn"]
>[2];

function broadcastProcessEvent<TConnParams>(
	c: AgentOsActionContext<TConnParams>,
	name: "processOutput" | "processExit",
	payload: unknown,
) {
	try {
		c.broadcast(name, payload);
	} catch (error) {
		if (error instanceof ActorStopping) {
			return;
		}
		throw error;
	}
}

// Build process execution actions for the actor factory.
export function buildProcessActions<TConnParams>(
	config: AgentOsActorConfig<TConnParams>,
) {
	return {
		exec: async (
			c: AgentOsActionContext<TConnParams>,
			command: string,
			options?: ExecOptions,
		): Promise<ExecResult> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.exec(command, options);
		},

		spawn: async (
			c: AgentOsActionContext<TConnParams>,
			command: string,
			args: string[],
			options?: SpawnOptions,
		): Promise<{ pid: number }> => {
			const agentOs = await ensureVm(c, config);
			const { pid } = agentOs.spawn(command, args, {
				...options,
				onStdout: (data: Uint8Array) => {
					broadcastProcessEvent(c, "processOutput", {
						pid,
						stream: "stdout" as const,
						data,
					});
					options?.onStdout?.(data);
				},
				onStderr: (data: Uint8Array) => {
					broadcastProcessEvent(c, "processOutput", {
						pid,
						stream: "stderr" as const,
						data,
					});
					options?.onStderr?.(data);
				},
			});

			c.vars.activeProcesses.add(pid);
			syncPreventSleep(c);
			c.log.info({
				msg: "agent-os process spawned",
				pid,
				command,
			});

			agentOs.waitProcess(pid)
				.then((exitCode) => {
					broadcastProcessEvent(c, "processExit", { pid, exitCode });
					c.log.info({
						msg: "agent-os process exited",
						pid,
						exitCode,
					});
				})
				.catch(() => {
					// Process killed during dispose. Silently clean up.
				})
				.finally(() => {
					c.vars.activeProcesses.delete(pid);
					syncPreventSleep(c);
				});

			return { pid };
		},

		writeProcessStdin: async (
			c: AgentOsActionContext<TConnParams>,
			pid: number,
			data: string | Uint8Array,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			agentOs.writeProcessStdin(pid, data);
		},

		closeProcessStdin: async (
			c: AgentOsActionContext<TConnParams>,
			pid: number,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			agentOs.closeProcessStdin(pid);
		},

		waitProcess: async (
			c: AgentOsActionContext<TConnParams>,
			pid: number,
		): Promise<number> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.waitProcess(pid);
		},

		listProcesses: async (
			c: AgentOsActionContext<TConnParams>,
		): Promise<SpawnedProcessInfo[]> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.listProcesses();
		},

		allProcesses: async (
			c: AgentOsActionContext<TConnParams>,
		): Promise<ProcessInfo[]> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.allProcesses();
		},

		processTree: async (
			c: AgentOsActionContext<TConnParams>,
		): Promise<ProcessTreeNode[]> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.processTree();
		},

		getProcess: async (
			c: AgentOsActionContext<TConnParams>,
			pid: number,
		): Promise<SpawnedProcessInfo> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.getProcess(pid);
		},

		stopProcess: async (
			c: AgentOsActionContext<TConnParams>,
			pid: number,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			agentOs.stopProcess(pid);
		},

		killProcess: async (
			c: AgentOsActionContext<TConnParams>,
			pid: number,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			agentOs.killProcess(pid);
		},
	};
}
