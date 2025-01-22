#!/usr/bin/env -S deno run -A

// This script can be used to call the internal build tooling directly on a
// given script. This is helpful for iterating on the esbuild config with
// isolated test cases without having to rebuild the whole toolchain.
//
// The build toolchain is fully tested e2e on all of the examples, so this is
// only intended for manual iteration.
//
// To run:
//
// ````
// cd path/to/deno/project
// /path/to/scripts/js_utils/build.ts name_of_script.ts
// ````
//
// This will output the output to `out/`.

import { resolve } from "@std/path";

const [entry] = Deno.args;
console.log("Building", entry);

const ROOT_DIR = resolve(import.meta.dirname!, "..", "..");
const JS_UTILS_DIR = resolve(ROOT_DIR, "packages/toolchain/js-utils-embed/js");

const input = {
	entryPoint: resolve(Deno.cwd(), entry),
	outDir: resolve(Deno.cwd(), "out"),
	deno: {
		//configPath: resolve(Deno.cwd(), "deno.jsonc"),
		//importMapUrl: z.string().optional(),
		lockPath: resolve(Deno.cwd(), "deno.lock"),
	},
	bundle: {
		minify: false,
		analyzeResult: true,
		logLevel: "debug",
	},
};

const output1 = await new Deno.Command("deno", {
	args: [
		"clean"
	],
	cwd: JS_UTILS_DIR,
	stdout: "inherit",
	stderr: "inherit",
}).output();

const output = await new Deno.Command("deno", {
	args: [
		"run",
		"-A",
		"src/tasks/build/mod.ts",
		"--input",
		JSON.stringify(input),
	],
	cwd: JS_UTILS_DIR,
	stdout: "inherit",
	stderr: "inherit",
}).output();
if (!output.success) {
	throw new Error("Failed");
}

console.log("output", output);
