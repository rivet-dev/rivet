import { importNodeDependencies } from "@/utils/node";
import { FileSystemActorDriver } from "./actor";
import { FileSystemGlobalState } from "./global-state";
import { FileSystemManagerDriver } from "./manager";
import { DriverConfig } from "@/registry/config";

export { FileSystemActorDriver } from "./actor";
export { FileSystemGlobalState } from "./global-state";
export { FileSystemManagerDriver } from "./manager";
export { getStoragePath } from "./utils";

export function createFileSystemOrMemoryDriver(
	persist: boolean = true,
	customPath?: string,
): DriverConfig {
	importNodeDependencies();

	const state = new FileSystemGlobalState(persist, customPath);
	const driverConfig: DriverConfig = {
		name: persist ? "file-system" : "memory",
		displayName: persist ? "File System" : "Memory",
		manager: (config) =>
			new FileSystemManagerDriver(
				config,
				state,
				driverConfig,
			),
		actor: (config, managerDriver, inlineClient) => {
			const actorDriver = new FileSystemActorDriver(
				config,
				managerDriver,
				inlineClient,
				state,
			);

			state.onRunnerStart(
				config,
				inlineClient,
				actorDriver,
			);

			return actorDriver;
		},
		autoStartActorDriver: true,
	};
	return driverConfig;
}

export function createFileSystemDriver(opts?: { path?: string }): DriverConfig {
	return createFileSystemOrMemoryDriver(true, opts?.path);
}

export function createMemoryDriver(): DriverConfig {
	return createFileSystemOrMemoryDriver(false);
}
