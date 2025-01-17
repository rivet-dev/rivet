import type { PackageJson, TsConfigJson } from "type-fest";
import { join, joinGlobs, dirname, basename } from "@std/path";
import { expandGlob } from "@std/fs";

type DenoFlavoredPackageJson = PackageJson & {
	deno?: {
		imports?: Record<string, string>;
		exports?: Record<string, string>;
		publish?: {
			include?: string[];
			exclude?: string[];
		};
		compilerOptions?: TsConfigJson.CompilerOptions;
	};
};

type InternalPackagesMap = Map<
	string,
	{ path: string; packageJson: DenoFlavoredPackageJson }
>;

interface Config {
	skipPathInInternalPackages?: string;
	internalPackagesLinkPath: string;
	cwd: string;
}
/**
 * This script will transform a package.json file to a deno.json file.
 * It will:
 * - Copy the name, version, description, and license fields
 * - Resolve dependencies to use the correct deno prefix (jsr: or npm:)
 * - Symlink internal packages to a directory
 * - Update the imports field to point to the symlinked packages
 * - Use `deno` field in exports to point to the correct path
 */
export async function transformPackageJsonToDenoJson(
	config: Config = { internalPackagesLinkPath: "internal", cwd: Deno.cwd() },
) {
	const packageJson: DenoFlavoredPackageJson = JSON.parse(
		await Deno.readTextFile(join(config.cwd,"package.json")),
	);

	const denoPkg = {
		...packageJson.deno,
		name: packageJson.name,
		version: packageJson.version,
		description: packageJson.description,
		license: packageJson.license,
		compilerOptions: packageJson.deno?.compilerOptions,
		imports: { ...(packageJson.deno?.imports ?? {}) },
		exports: { ...(packageJson.deno?.exports ?? {}) },
		publish: { ...(packageJson.deno?.publish ?? {}) },
	};

	const dirStat = await Deno.stat(config.internalPackagesLinkPath).catch(
		() => null,
	);

	if (dirStat?.isDirectory) {
		await Deno.remove(config.internalPackagesLinkPath, { recursive: true });
	}

	const internalPackages = await getInternalPackages(config.cwd);

	// Copy exports from package.json to deno.json
	for (const [exportPath, paths] of Object.entries<PackageJson.Exports>(
		packageJson.exports || {},
	)) {
		denoPkg.exports = denoPkg.exports || {};
		denoPkg.exports[exportPath] = (paths as PackageJson.ExportConditions)
			?.deno as string;
	}

	const dependencies = listAllDepedencies(packageJson, internalPackages);

	for (const [pkgName, pkgVersion] of Object.entries(dependencies)) {
		denoPkg.imports = denoPkg.imports || {};

		// If the package comes from jsr, use jsr prefix
		if (pkgVersion.startsWith("npm:@jsr/")) {
			const [, correctVersion] = pkgVersion
				.replace("npm:@jsr/", "")
				.split("@");

			denoPkg.imports[pkgName] = `jsr:${pkgName}@${correctVersion}`;
		}
		// If the package is internal, symlink it, and update the imports
		else if (pkgVersion.startsWith("workspace:")) {
			const internalPkg = internalPackages.get(pkgName);

			if (!internalPkg) {
				throw new Error(`Package ${pkgName} not found in workspaces`);
			}

			if (!internalPkg.packageJson.name) {
				throw new Error(
					`Package ${pkgName} is missing a name field in its package.json`,
				);
			}

			const link = basename(dirname(internalPkg.path));

			await Deno.mkdir(
				join(config.cwd, config.internalPackagesLinkPath),
				{ recursive: true },
			);

			const internalPkgPath = join(
				internalPkg.path,
				"..",
				config.skipPathInInternalPackages || "",
			);
			const relativeAliasedPkgPath = join(
				config.internalPackagesLinkPath,
				link,
			);

			const aliasedPkgPath = join(config.cwd, relativeAliasedPkgPath);

			await Deno.symlink(internalPkgPath, aliasedPkgPath);

			// Add the symlinked package to the negation of exclude (exclude it from exclude)
			if (
				!denoPkg.publish?.exclude?.includes(
					`!${relativeAliasedPkgPath}`,
				)
			) {
				denoPkg.publish.exclude = denoPkg.publish.exclude || [];
				denoPkg.publish.exclude = [
					...denoPkg.publish.exclude,
					`!${relativeAliasedPkgPath}`,
				];
			}

			const internalPkgExports = internalPkg.packageJson.exports || {};

			for (const [exportPath, paths] of Object.entries(
				internalPkgExports,
			)) {
				if (
					typeof paths === "string" ||
					paths === null ||
					Array.isArray(paths)
				) {
					throw new Error(
						`Package ${pkgName} has invalid exports field in its package.json (expected object, got ${typeof paths})`,
					);
				}

				const exported = join(internalPkg.packageJson.name, exportPath);

				if (!paths?.deno) {
					throw new Error(
						`Missing "deno" field in exports for ${exported}`,
					);
				}

				if (typeof paths.deno !== "string") {
					throw new Error(
						`Invalid "deno" field in exports for ${exported} (expected object, got ${typeof paths.deno})`,
					);
				}

				const newPath = join(
					"./",
					"internal",
					link,
					paths.deno.replace(
						config.skipPathInInternalPackages || "",
						"",
					),
				);

				denoPkg.imports[exported] = `./${newPath}`;
			}
		} else {
			// Otherwise, use npm prefix
			denoPkg.imports[pkgName] = `npm:${pkgName}@${pkgVersion}`;
		}
	}

	await Deno.writeTextFile(join(config.cwd, "deno.json"), JSON.stringify(denoPkg, null, "\t"));
}

async function findRootWorkspace(dir = Deno.cwd()): Promise<string> {
	try {
		const { workspaces } = JSON.parse(
			await Deno.readTextFile(join(dir, "package.json")),
		);
		if (workspaces) {
			return join(dir, "package.json");
		}
		return findRootWorkspace(join(dir, ".."));
	} catch {
		return findRootWorkspace(join(dir, ".."));
	}
}

async function getInternalPackages(cwd: string): Promise<InternalPackagesMap> {
	const workspaceRoot = await findRootWorkspace(cwd);
	const { workspaces } = JSON.parse(await Deno.readTextFile(workspaceRoot));

	const globPatterns = workspaces.map((pattern: string) =>
		joinGlobs([pattern, "package.json"]),
	);

	const internalPackages = new Map<
		string,
		{ path: string; packageJson: DenoFlavoredPackageJson }
	>();

	for (const pattern of globPatterns) {
		for await (const entry of expandGlob(pattern, {
			root: join(workspaceRoot, ".."),
		})) {
			const packageJson = JSON.parse(await Deno.readTextFile(entry.path));

			internalPackages.set(packageJson.name, {
				path: entry.path,
				packageJson,
			});
		}
	}

	return internalPackages;
}

function listAllDepedencies(
	pkg: DenoFlavoredPackageJson,
	internalPackages: InternalPackagesMap,
): Record<string, string> {
	return {
		...(pkg.dependencies || {}),
		...(pkg.optionalDependencies || {}),
		...(pkg.peerDependencies || {}),
		...Object.fromEntries(
			Object.entries(pkg.devDependencies || {}).filter(([name]) =>
				internalPackages.has(name),
			),
		),
	} as Record<string, string>;
}