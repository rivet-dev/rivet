import { z } from "zod";
import type { DriverConfig } from "@/registry/config";
import { importNodeDependencies } from "@/utils/node";
import { FileSystemActorDriver } from "./actor";
import {
	type FileSystemDriverOptions,
	FileSystemGlobalState,
} from "./global-state";
import { FileSystemManagerDriver } from "./manager";

export { FileSystemActorDriver } from "./actor";
export { FileSystemGlobalState } from "./global-state";
export { FileSystemManagerDriver } from "./manager";
export { getStoragePath } from "./utils";

const CreateFileSystemDriverOptionsSchema = z.object({
	/** Custom path for storage. */
	path: z.string().optional(),
});

type CreateFileSystemDriverOptionsInput = z.input<
	typeof CreateFileSystemDriverOptionsSchema
>;

export function createFileSystemOrMemoryDriver(
	persist: boolean = true,
	options?: CreateFileSystemDriverOptionsInput,
): DriverConfig {
	importNodeDependencies();

	const stateOptions: FileSystemDriverOptions = {
		persist,
		customPath: options?.path,
	};
	const state = new FileSystemGlobalState(stateOptions);
	const driverConfig: DriverConfig = {
		name: persist ? "file-system" : "memory",
		displayName: persist ? "File System" : "Memory",
		manager: (config) =>
			new FileSystemManagerDriver(config, state, driverConfig),
		actor: (config, managerDriver, inlineClient) => {
			const actorDriver = new FileSystemActorDriver(
				config,
				managerDriver,
				inlineClient,
				state,
			);

			state.onRunnerStart(config, inlineClient, actorDriver);

			return actorDriver;
		},
		autoStartActorDriver: true,
	};
	return driverConfig;
}

export function createFileSystemDriver(
	opts?: CreateFileSystemDriverOptionsInput,
): DriverConfig {
	const validatedOpts = opts
		? CreateFileSystemDriverOptionsSchema.parse(opts)
		: undefined;
	return createFileSystemOrMemoryDriver(true, validatedOpts);
}

export function createMemoryDriver(): DriverConfig {
	return createFileSystemOrMemoryDriver(false);
}
