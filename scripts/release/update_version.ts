import type { ReleaseOpts } from "./main.ts";
import { glob } from "glob";
import $ from "dax";
import { assert } from "@std/assert";

export async function updateVersion(opts: ReleaseOpts) {
	// Define substitutions
	const findReplace = [
		{
			path: "Cargo.toml",
			find: /\[workspace\.package\]\nversion = ".*"/,
			replace: `[workspace.package]\nversion = "${opts.version}"`,
		},
		{
			path: "sdks/actor/*/package.json",
			find: /"version": ".*"/,
			replace: `"version": "${opts.version}"`,
		},
		{
			path: "sdks/api/*/typescript/package.json",
			find: /"version": ".*"/,
			replace: `"version": "${opts.version}"`,
		},
		{
			path: "sdks/api/fern/definition/api.yml",
			find: /version:\n\s\sheader: "X-API-Version"\n\s\sdefault: ".*"\n\s\svalues: \[".*"\]/,
			replace: `version:\n  header: "X-API-Version"\n  default: "${opts.version}"\n  values: ["${opts.version}"]`,
		},
	];

	// Substitute all files
	for (const { path: globPath, find, replace } of findReplace) {
		const paths = await glob(globPath, { cwd: opts.root });
		assert(paths.length > 0, `no paths matched: ${globPath}`);
		for (const path of paths) {
			const file = await Deno.readTextFile(path);
			assert(find.test(file), `file does not match ${find}: ${path}`);
			const newFile = file.replace(find, replace);
			await Deno.writeTextFile(path, newFile);

			await $`git add ${path}`.cwd(opts.root);
		}
	}
}
