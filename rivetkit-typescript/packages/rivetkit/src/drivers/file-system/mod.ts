import { z } from "zod";
import type { DriverConfig } from "@/registry/run-config";
import { importNodeDependencies } from "@/utils/node";
import { FileSystemActorDriver } from "./actor";
import {
	FileSystemGlobalState,
	type FileSystemDriverOptions,
} from "./global-state";
import { FileSystemManagerDriver } from "./manager";
import { DriverConfig } from "@/registry/config";

export { FileSystemActorDriver } from "./actor";
export { FileSystemGlobalState } from "./global-state";
export { FileSystemManagerDriver } from "./manager";
export { getStoragePath } from "./utils";

const CreateFileSystemDriverOptionsSchema = z.object({
	/** Custom path for storage. */
	path: z.string().optional(),
	/**
	 * Use native SQLite (better-sqlite3) instead of KV-backed SQLite.
	 * Requires better-sqlite3 to be installed.
	 * @default false
	 */
	useNativeSqlite: z.boolean().optional().default(false),
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
		useNativeSqlite: options?.useNativeSqlite ?? false,
	};
	const state = new FileSystemGlobalState(stateOptions);
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
