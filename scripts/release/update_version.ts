import * as fs from "node:fs/promises";
import * as pathModule from "node:path";
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
		{
			path: "rivetkit-typescript/packages/sqlite-native/npm/*/package.json",
			find: /"version": ".*"/,
			replace: `"version": "${opts.version}"`,
		},
		{
			path: "rivetkit-typescript/packages/sqlite-native/package.json",
			find: /("@rivetkit\/sqlite-native-[^"]+": )"[^"]+"/g,
			replace: `$1"${opts.version}"`,
		},
		{
			path: "rivetkit-typescript/packages/sqlite-native/Cargo.toml",
			find: /^version = ".*"/m,
			replace: `version = "${opts.version}"`,
		},
		{
			path: "examples/**/package.json",
			find: /"(@rivetkit\/[^"]+|rivetkit)": "\^?[0-9]+\.[0-9]+\.[0-9]+(?:-[^"]+)?"/g,
			replace: `"$1": "^${opts.version}"`,
			required: false,
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
	for (const { path: globPath, find, replace, required = true } of findReplace) {
		const paths = await glob(globPath, { cwd: opts.root });
		assert(paths.length > 0, `no paths matched: ${globPath}`);
		for (const fileRelPath of paths) {
			const filePath = pathModule.join(opts.root, fileRelPath);
			const file = await fs.readFile(filePath, "utf-8");

			find.lastIndex = 0;
			const hasMatch = find.test(file);
			if (!hasMatch) {
				if (required) {
					assert(false, `file does not match ${find}: ${fileRelPath}`);
				}

				continue;
			}

			find.lastIndex = 0;
			const newFile = file.replace(find, replace);
			await fs.writeFile(filePath, newFile);

			await $({ cwd: opts.root })`git add ${fileRelPath}`;
		}
	}
}
