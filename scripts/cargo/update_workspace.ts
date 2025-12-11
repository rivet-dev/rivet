#!/usr/bin/env tsx

import { readFile, writeFile, access } from 'node:fs/promises';
import { join, relative, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse, stringify } from '@iarna/toml';
import fg from 'fast-glob';

const __dirname = dirname(fileURLToPath(import.meta.url));
const rootDir = join(__dirname, "../..");

async function exists(path: string): Promise<boolean> {
	try {
		await access(path);
		return true;
	} catch {
		return false;
	}
}

async function updateCargoToml() {
	const workspaceTomlPath = join(rootDir, "Cargo.toml");
	const workspaceTomlContent = await readFile(workspaceTomlPath, 'utf-8');
	const workspaceToml = parse(workspaceTomlContent) as any;

	// Find all Cargo.toml files in engine/packages/* (1 level deep)
	const enginePackages = await fg('engine/packages/*/Cargo.toml', {
		cwd: rootDir,
		ignore: ['**/node_modules/**'],
	});

	// Find all Cargo.toml files in engine/sdks/rust/* (1 level deep) if it exists
	const sdksRustDir = join(rootDir, "engine", "sdks", "rust");
	let sdkPackages: string[] = [];
	if (await exists(sdksRustDir)) {
		sdkPackages = await fg('engine/sdks/rust/*/Cargo.toml', {
			cwd: rootDir,
			ignore: ['**/node_modules/**'],
		});
	}

	const allCargoTomls = [...enginePackages, ...sdkPackages];

	// Build list of workspace members
	const members: string[] = [];
	for (const cargoTomlPath of allCargoTomls) {
		const packagePath = cargoTomlPath.replace(/\/Cargo\.toml$/, "");
		members.push(packagePath);
	}

	// Sort members
	members.sort();

	// Remove path dependencies, since we'll replace these. This lets us
	// preserve existing external dependencies.
	const existingDependencies = workspaceToml.workspace?.dependencies || {};
	for (const [name, dep] of Object.entries(existingDependencies)) {
		if (dep && typeof dep === "object" && "path" in dep) {
			delete existingDependencies[name];
		}
	}

	// Build new workspace dependencies
	const newDependencies: Record<string, any> = {};
	const packageAliases: Record<string, string[]> = {
		"rivet-util": ["util"],
		gasoline: ["gas"],
	};

	for (const packagePath of members) {
		const packageTomlPath = join(rootDir, packagePath, "Cargo.toml");
		const packageTomlContent = await readFile(packageTomlPath, 'utf-8');
		const packageToml = parse(packageTomlContent) as any;

		// Save to workspace
		newDependencies[packageToml.package.name] = {
			path: packagePath,
		};

		// Register package alias names with the workspace
		if (packageToml.package.name in packageAliases) {
			for (const alias of packageAliases[packageToml.package.name]) {
				newDependencies[alias] = {
					package: packageToml.package.name,
					path: packagePath,
				};
			}
		}

		// // Replace all package dependencies that refer to a workspace package to use `*.workspace = true`
		// for (
		// 	const [depName, dep] of Object.entries(packageToml.dependencies || {})
		// ) {
		// 	if (dep && typeof dep === "object" && "path" in dep) {
		// 		const depAbsolutePath = join(packagePath, dep.path);
		// 		const depRelativePath = relative(rootDir, depAbsolutePath);
		// 		if (members.includes(depRelativePath)) {
		// 			delete packageToml.dependencies[depName].path;
		// 			packageToml.dependencies[depName].workspace = true;
		// 		}
		// 	}
		// }

		// // Write the updated package Cargo.toml
		// const updatedPackageTomlContent = stringify(packageToml);
		// await writeFile(packageTomlPath, updatedPackageTomlContent, 'utf-8');
	}

	// Update and write workspace
	workspaceToml.workspace = workspaceToml.workspace || {};
	workspaceToml.workspace.members = members;
	workspaceToml.workspace.dependencies = {
		...existingDependencies,
		...newDependencies,
	};

	const updatedTomlContent = stringify(workspaceToml);
	await writeFile(workspaceTomlPath, updatedTomlContent, 'utf-8');
}

updateCargoToml().catch(console.error);
