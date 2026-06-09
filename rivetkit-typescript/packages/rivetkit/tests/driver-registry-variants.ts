import { join } from "node:path";

export interface DriverRegistryVariant {
	name: "static" | "worker" | "dynamic";
	registryPath: string;
	skip: boolean;
	skipReason?: string;
	/**
	 * Bridged variants run user code in per-actor worker threads, which only
	 * the native runtime supports.
	 */
	nativeOnly?: boolean;
}

export function getDriverRegistryVariants(
	currentDir: string,
): DriverRegistryVariant[] {
	return [
		{
			name: "static",
			registryPath: join(
				currentDir,
				"../fixtures/driver-test-suite/registry-static.ts",
			),
			skip: false,
		},
		{
			name: "worker",
			registryPath: join(
				currentDir,
				"../fixtures/driver-test-suite/registry-worker.ts",
			),
			skip: false,
			nativeOnly: true,
		},
		{
			name: "dynamic",
			registryPath: join(
				currentDir,
				"../fixtures/driver-test-suite/registry-dynamic.ts",
			),
			skip: false,
			nativeOnly: true,
		},
	];
}
