import { existsSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

export interface DriverRegistryVariant {
	name: "static" | "dynamic";
	registryPath: string;
	skip: boolean;
	skipReason?: string;
}

const SECURE_EXEC_DIST_CANDIDATE_PATHS = [
	join(
		process.env.HOME ?? "",
		"secure-exec-rivet/packages/secure-exec/dist/index.js",
	),
	join(
		process.env.HOME ?? "",
		"secure-exec-rivet/packages/sandboxed-node/dist/index.js",
	),
];

function resolveSecureExecDistPath(): string | undefined {
	for (const candidatePath of SECURE_EXEC_DIST_CANDIDATE_PATHS) {
		if (existsSync(candidatePath)) {
			return candidatePath;
		}
	}
	return undefined;
}

function getDynamicVariantSkipReason(): string | undefined {
	if (process.env.RIVETKIT_DRIVER_TEST_SKIP_DYNAMIC_IN_DYNAMIC === "1") {
		return "Dynamic registry parity is skipped for this nested dynamic harness only. We still target full static and dynamic runtime compatibility for all normal driver suites.";
	}

	if (process.env.RIVETKIT_DYNAMIC_SECURE_EXEC_SPECIFIER) {
		return undefined;
	}

	const secureExecDistPath = resolveSecureExecDistPath();
	if (!secureExecDistPath) {
		return `Dynamic registry parity requires secure-exec dist at one of: ${SECURE_EXEC_DIST_CANDIDATE_PATHS.join(", ")}.`;
	}

	process.env.RIVETKIT_DYNAMIC_SECURE_EXEC_SPECIFIER = pathToFileURL(
		secureExecDistPath,
	).href;

	return undefined;
}

export function getDriverRegistryVariants(currentDir: string): DriverRegistryVariant[] {
	const dynamicSkipReason = getDynamicVariantSkipReason();

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
			name: "dynamic",
			registryPath: join(
				currentDir,
				"../fixtures/driver-test-suite/registry-dynamic.ts",
			),
			skip: dynamicSkipReason !== undefined,
			skipReason: dynamicSkipReason,
		},
	];
}
