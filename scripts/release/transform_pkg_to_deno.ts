import type { PackageJson, TsConfigJson } from "type-fest";
import * as path from "node:path";
import * as fs from "node:fs/promises";
import { glob } from "glob";

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
	config: Config = { internalPackagesLinkPath: "internal", cwd: process.cwd() },
) {
	const packageJson: DenoFlavoredPackageJson = JSON.parse(
		await fs.readFile(path.join(config.cwd,"package.json"), 'utf-8'),
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

	try {
		const dirStat = await fs.stat(config.internalPackagesLinkPath);
		if (dirStat.isDirectory()) {
			await fs.rm(config.internalPackagesLinkPath, { recursive: true });
		}
	} catch (error) {
		// Directory doesn't exist, that's fine
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

			const link = path.basename(path.dirname(internalPkg.path));

			await fs.mkdir(
				path.join(config.cwd, config.internalPackagesLinkPath),
				{ recursive: true },
			);

			const internalPkgPath = path.join(
				internalPkg.path,
				"..",
				config.skipPathInInternalPackages || "",
			);
			const relativeAliasedPkgPath = path.join(
				config.internalPackagesLinkPath,
				link,
			);

			const aliasedPkgPath = path.join(config.cwd, relativeAliasedPkgPath);

			await fs.symlink(internalPkgPath, aliasedPkgPath);

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

				const exported = path.join(internalPkg.packageJson.name, exportPath);

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

				const newPath = path.join(
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

	await fs.writeFile(path.join(config.cwd, "deno.json"), JSON.stringify(denoPkg, null, "\t"));
}

async function findRootWorkspace(dir = process.cwd()): Promise<string> {
	try {
		const { workspaces } = JSON.parse(
			await fs.readFile(path.join(dir, "package.json"), 'utf-8'),
		);
		if (workspaces) {
			return path.join(dir, "package.json");
		}
		return findRootWorkspace(path.join(dir, ".."));
	} catch {
		return findRootWorkspace(path.join(dir, ".."));
	}
}

async function getInternalPackages(cwd: string): Promise<InternalPackagesMap> {
	const workspaceRoot = await findRootWorkspace(cwd);
	const { workspaces } = JSON.parse(await fs.readFile(workspaceRoot, 'utf-8'));

	const internalPackages = new Map<
		string,
		{ path: string; packageJson: DenoFlavoredPackageJson }
	>();

	for (const pattern of workspaces) {
		const globPattern = path.join(pattern, "package.json");
		const matches = await glob(globPattern, {
			cwd: path.join(workspaceRoot, ".."),
			absolute: true,
		});

		for (const matchPath of matches) {
			const packageJson = JSON.parse(await fs.readFile(matchPath, 'utf-8'));

			internalPackages.set(packageJson.name, {
				path: matchPath,
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