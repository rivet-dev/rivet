import type { DriverConfig } from "@/registry/run-config";
import { importNodeDependencies } from "@/utils/node";
import { FileSystemActorDriver } from "./actor";
import { FileSystemGlobalState } from "./global-state";
import { FileSystemManagerDriver } from "./manager";

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
		manager: (registryConfig, runConfig) =>
			new FileSystemManagerDriver(
				registryConfig,
				runConfig,
				state,
				driverConfig,
			),
		actor: (registryConfig, runConfig, managerDriver, inlineClient) => {
			const actorDriver = new FileSystemActorDriver(
				registryConfig,
				runConfig,
				managerDriver,
				inlineClient,
				state,
			);

			state.onRunnerStart(
				registryConfig,
				runConfig,
				inlineClient,
				actorDriver,
			);

			return actorDriver;
		},
	};
	return driverConfig;
}

export function createFileSystemDriver(opts?: { path?: string }): DriverConfig {
	return createFileSystemOrMemoryDriver(true, opts?.path);
}

export function createMemoryDriver(): DriverConfig {
	return createFileSystemOrMemoryDriver(false);
}
