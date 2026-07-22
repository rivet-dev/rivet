/**
 * Overview: Interactive shell actions for the agent-os HOC. Bridges
 * 0.2.8 `ShellData` events onto the actor `shellData` broadcast.
 */
import type { OpenShellOptions, ShellData } from "@rivet-dev/agent-os-core";
import type { AgentOsActorConfig } from "../config";
import type { AgentOsActionContext } from "../types";
import { ensureVm, syncPreventSleep } from "./index";

// Build shell actions for the actor factory.
export function buildShellActions<TConnParams>(
	config: AgentOsActorConfig<TConnParams>,
) {
	return {
		openShell: async (
			c: AgentOsActionContext<TConnParams>,
			options?: OpenShellOptions,
		): Promise<{ shellId: string }> => {
			const agentOs = await ensureVm(c, config);
			const { shellId } = agentOs.openShell(options);

			// 0.2.8 delivers { shellId, data } instead of bare Uint8Array.
			agentOs.onShellData(shellId, (event: ShellData) => {
				c.broadcast("shellData", {
					shellId: event.shellId,
					data: event.data,
				});
			});

			c.vars.activeShells.add(shellId);
			syncPreventSleep(c);
			c.log.info({ msg: "agent-os shell opened", shellId });

			return { shellId };
		},

		writeShell: async (
			c: AgentOsActionContext<TConnParams>,
			shellId: string,
			data: string | Uint8Array,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			agentOs.writeShell(shellId, data);
		},

		resizeShell: async (
			c: AgentOsActionContext<TConnParams>,
			shellId: string,
			cols: number,
			rows: number,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			agentOs.resizeShell(shellId, cols, rows);
		},

		closeShell: async (
			c: AgentOsActionContext<TConnParams>,
			shellId: string,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			agentOs.closeShell(shellId);
			c.vars.activeShells.delete(shellId);
			syncPreventSleep(c);
			c.log.info({ msg: "agent-os shell closed", shellId });
		},
	};
}
