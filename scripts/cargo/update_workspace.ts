#!/usr/bin/env -S deno run --allow-net --allow-env --allow-read --allow-write

import { parse, stringify } from "@std/toml";
import { walk, exists } from "@std/fs";
import { join, relative } from "@std/path";

const rootDir = join(import.meta.dirname, "../..");

async function updateCargoToml() {
	const workspaceTomlPath = join(rootDir, "Cargo.toml");
	const workspaceTomlContent = await Deno.readTextFile(workspaceTomlPath);
	const workspaceToml = parse(workspaceTomlContent);

	const entries = async function*() {
		for await (const entry of walk(join(rootDir, "packages"), {
			includeDirs: false,
			exts: ["toml"],
			skip: [/node_modules/],
		})) {
			yield entry;
		}

		// Yield from OSS
		if (await exists(join(rootDir, "oss"))) {
			for await (const entry of walk(join(rootDir, "oss", "packages"), {
				includeDirs: false,
				exts: ["toml"],
			})) {
				yield entry;
			}
		}
	}();

	// Find all workspace members
	const members: string[] = [];
	for await (
		const entry of entries
	) {
		// Exclude infra packages
		if (
			entry.path.includes("packages/infra/client") ||
			entry.path.includes("packages/infra/job-runner")
		) {
			continue;
		}

		const packagePath = relative(
			rootDir,
			entry.path.replace(/\/Cargo\.toml$/, ""),
		);
		members.push(packagePath);
	}

	// Hardcode extra workspace members
	members.push("sdks/api/full/rust");

	// Sort deps
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
		"rivet-util-mm": ["util-mm"],
		"rivet-util-job": ["util-job"],
		"rivet-util-search": ["util-search"],
		"rivet-util-captcha": ["util-captcha"],
		"rivet-util-cdn": ["util-cdn"],
		"rivet-util-team": ["util-team"],
		"rivet-util-build": ["util-build"],
	};
	for (const packagePath of members) {
		const packageTomlPath = join(rootDir, packagePath, "Cargo.toml");
		const packageTomlContent = await Deno.readTextFile(packageTomlPath);
		const packageToml = parse(packageTomlContent);

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
		// await Deno.writeTextFile(packageTomlPath, updatedPackageTomlContent);
	}

	// Update and write workspace
	workspaceToml.workspace = workspaceToml.workspace || {};
	workspaceToml.workspace.members = members;
	workspaceToml.workspace.dependencies = {
		...existingDependencies,
		...newDependencies,
	};

	const updatedTomlContent = stringify(workspaceToml);
	await Deno.writeTextFile(workspaceTomlPath, updatedTomlContent);
}

updateCargoToml().catch(console.error);
