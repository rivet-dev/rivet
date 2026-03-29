import type {
	AgentRegistryEntry,
	BatchReadResult,
	BatchWriteEntry,
	BatchWriteResult,
	DirEntry,
	ReaddirRecursiveOptions,
} from "@rivet-dev/agent-os-core";
import type { AgentOsActorConfig } from "../config";
import type { AgentOsActionContext } from "../types";
import { ensureVm } from "./index";

// Infer types from AgentOs methods since @secure-exec/core is not a direct dep.
type VirtualStat = Awaited<
	ReturnType<import("@rivet-dev/agent-os-core").AgentOs["stat"]>
>;
type DeleteOptions = Parameters<
	import("@rivet-dev/agent-os-core").AgentOs["delete"]
>[1];

// Build filesystem and agent registry actions for the actor factory.
export function buildFilesystemActions<TConnParams>(
	config: AgentOsActorConfig<TConnParams>,
) {
	return {
		readFile: async (
			c: AgentOsActionContext<TConnParams>,
			path: string,
		): Promise<Uint8Array> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.readFile(path);
		},

		writeFile: async (
			c: AgentOsActionContext<TConnParams>,
			path: string,
			content: string | Uint8Array,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			await agentOs.writeFile(path, content);
		},

		readFiles: async (
			c: AgentOsActionContext<TConnParams>,
			paths: string[],
		): Promise<BatchReadResult[]> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.readFiles(paths);
		},

		writeFiles: async (
			c: AgentOsActionContext<TConnParams>,
			entries: BatchWriteEntry[],
		): Promise<BatchWriteResult[]> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.writeFiles(entries);
		},

		mkdir: async (
			c: AgentOsActionContext<TConnParams>,
			path: string,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			await agentOs.mkdir(path);
		},

		readdir: async (
			c: AgentOsActionContext<TConnParams>,
			path: string,
		): Promise<string[]> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.readdir(path);
		},

		readdirRecursive: async (
			c: AgentOsActionContext<TConnParams>,
			path: string,
			options?: ReaddirRecursiveOptions,
		): Promise<DirEntry[]> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.readdirRecursive(path, options);
		},

		stat: async (
			c: AgentOsActionContext<TConnParams>,
			path: string,
		): Promise<VirtualStat> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.stat(path);
		},

		exists: async (
			c: AgentOsActionContext<TConnParams>,
			path: string,
		): Promise<boolean> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.exists(path);
		},

		move: async (
			c: AgentOsActionContext<TConnParams>,
			from: string,
			to: string,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			await agentOs.move(from, to);
		},

		deleteFile: async (
			c: AgentOsActionContext<TConnParams>,
			path: string,
			options?: DeleteOptions,
		): Promise<void> => {
			const agentOs = await ensureVm(c, config);
			await agentOs.delete(path, options);
		},

		// TODO: mountFs and unmountFs are not exposed as actor actions because
		// filesystem drivers (VirtualFileSystem) are not serializable over the
		// network. Mount filesystems via the `options.mounts` config in agentOs()
		// instead. See: https://github.com/rivet-dev/rivet/issues/XXXX

		listAgents: async (
			c: AgentOsActionContext<TConnParams>,
		): Promise<AgentRegistryEntry[]> => {
			const agentOs = await ensureVm(c, config);
			return agentOs.listAgents();
		},
	};
}
