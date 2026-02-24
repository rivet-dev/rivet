import * as fs from "node:fs/promises";
import * as path from "node:path";
import { $ } from "execa";
import { glob } from "glob";
import type { ReleaseOpts } from "./main";

function assert(condition: any, message?: string): asserts condition {
	if (!condition) {
		throw new Error(message || "Assertion failed");
	}
}

export async function updateVersion(opts: ReleaseOpts) {
	// Define substitutions
	const findReplace = [
		{
			path: "Cargo.toml",
			find: /([ \t]*)\[workspace\.package\]\n\1version = ".*"/,
			replace: `$1[workspace.package]\n$1version = "${opts.version}"`,
		},
		{
			path: "frontend/packages/*/package.json",
			find: /"version": ".*"/,
			replace: `"version": "${opts.version}"`,
		},
		{
			path: "engine/sdks/typescript/*/package.json",
			find: /"version": ".*"/,
			replace: `"version": "${opts.version}"`,
		},
		{
			path: "rivetkit-typescript/packages/*/package.json",
			find: /"version": ".*"/,
			replace: `"version": "${opts.version}"`,
		},
		// TODO: Update docs with pinned version
		// {
		// 	path: "site/src/content/docs/cloud/install.mdx",
		// 	find: /rivet-cli@.*/g,
		// 	replace: `rivet-cli@${opts.version}`,
		// },
		// {
		// 	path: "site/src/content/docs/cloud/install.mdx",
		// 	find: /RIVET_CLI_VERSION=.*/g,
		// 	replace: `RIVET_CLI_VERSION=${opts.version}`,
		// },
		// {
		// 	path: "site/src/content/docs/cloud/install.mdx",
		// 	find: /\$env:RIVET_CLI_VERSION = ".*"/g,
		// 	replace: `$env:RIVET_CLI_VERSION = "${opts.version}"`,
		// },
	];

	// Substitute all files
	for (const { path: globPath, find, replace } of findReplace) {
		const paths = await glob(globPath, { cwd: opts.root });
		assert(paths.length > 0, `no paths matched: ${globPath}`);
		for (const path of paths) {
			const file = await fs.readFile(path, "utf-8");
			assert(find.test(file), `file does not match ${find}: ${path}`);
			const newFile = file.replace(find, replace);
			await fs.writeFile(path, newFile);

			await $({ cwd: opts.root })`git add ${path}`;
		}
	}
}
