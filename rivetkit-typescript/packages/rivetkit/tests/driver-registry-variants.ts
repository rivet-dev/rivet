import { existsSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
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

function scorePnpmSecureExecEntry(entryName: string): number {
	return entryName.includes("pkg.pr.new") ? 1 : 0;
}

function resolveSecureExecDistPath(): string | undefined {
	for (const candidatePath of SECURE_EXEC_DIST_CANDIDATE_PATHS) {
		if (existsSync(candidatePath)) {
			return candidatePath;
		}
	}

	let current = process.cwd();
	while (true) {
		const virtualStoreDir = join(current, "node_modules/.pnpm");
		if (existsSync(virtualStoreDir)) {
			const entries = readdirSync(virtualStoreDir, {
				withFileTypes: true,
			}).sort(
				(a, b) =>
					scorePnpmSecureExecEntry(b.name) -
						scorePnpmSecureExecEntry(a.name) ||
					a.name.localeCompare(b.name),
			);

			for (const entry of entries) {
				if (!entry.isDirectory()) {
					continue;
				}

				for (const packageName of ["secure-exec", "sandboxed-node"]) {
					const candidatePath = join(
						virtualStoreDir,
						entry.name,
						"node_modules",
						packageName,
						"dist/index.js",
					);
					if (existsSync(candidatePath)) {
						return candidatePath;
					}
				}
			}
		}

		const parent = dirname(current);
		if (parent === current) {
			break;
		}
		current = parent;
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
	return [
		{
			name: "static",
			registryPath: join(
				currentDir,
				"../fixtures/driver-test-suite/registry-static.ts",
			),
			skip: false,
		},
		// TODO: Re-enable the dynamic registry variant after the static driver
		// suite is fully stabilized. Keep the dynamic files and skip-reason
		// plumbing in place so we can restore this entry cleanly later.
		// {
		// 	name: "dynamic",
		// 	registryPath: join(
		// 		currentDir,
		// 		"../fixtures/driver-test-suite/registry-dynamic.ts",
		// 	),
		// 	skip: dynamicSkipReason !== undefined,
		// 	skipReason: dynamicSkipReason,
		// },
	];
}
