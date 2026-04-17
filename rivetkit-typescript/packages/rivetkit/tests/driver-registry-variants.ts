import { join } from "node:path";

export interface DriverRegistryVariant {
	name: "static";
	registryPath: string;
	skip: boolean;
	skipReason?: string;
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
	];
}
