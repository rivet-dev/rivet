import type { AgentOs, ProcessHandle } from "../../src/index.js";

/**
 * Create a ProcessHandle from the flat API (AgentOs + pid).
 * Used in tests to pass to AcpClient without depending on ManagedProcess.
 */
export function createFlatProcessHandle(
	vm: AgentOs,
	pid: number,
): ProcessHandle {
	return {
		writeStdin: (data) => vm.writeProcessStdin(pid, data),
		kill: () => vm.killProcess(pid),
		wait: () => vm.waitProcess(pid),
	};
}
