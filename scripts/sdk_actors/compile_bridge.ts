#!/usr/bin/env -S deno run -A

import $ from "dax";
import { copy, exists, walk } from "@std/fs";
import { resolve } from "@std/path";

const ACTORS_SDK_PATH = resolve(
	import.meta.dirname,
	"../../sdks/actors",
);

const ACTOR_BRIDGE_PATH = resolve(ACTORS_SDK_PATH, "bridge");
const ACTOR_BRIDGE_TYPES_PATH = resolve(ACTOR_BRIDGE_PATH, "types");

const ACTOR_CORE_TYPES_PATH = resolve(ACTORS_SDK_PATH, "core", "src", "types");

// Compile JS bridge
await $`npx -p typescript@5.7.2 tsc -p tsconfig.bridge.json`
	.cwd(ACTOR_BRIDGE_PATH);

// Add header to JS bridge
for await (
	const entry of walk(
		resolve(
			import.meta.dirname,
			"../../packages/infra/client/isolate-v8-runner/js/",
		),
		{
			exts: [".js"],
			includeDirs: false,
		},
	)
) {
	const content = await Deno.readTextFile(entry.path);
	await Deno.writeTextFile(
		entry.path,
		"// DO NOT MODIFY\n//\n// Generated with scripts/pegboard/compile_bridge.ts\n\n" +
			content,
	);
}

// Clean types
if (await exists(ACTOR_BRIDGE_TYPES_PATH, { directory: true })) {
	await Deno.remove(ACTOR_BRIDGE_TYPES_PATH, { recursive: true });
}

// Compile TypeScript types
await $`npx -p typescript@5.7.2 tsc -p tsconfig.types.json`
	.cwd(ACTOR_BRIDGE_PATH);

// Replace imports from `ext:*` to `./*.d.ts`. This is required for this
// package to be usable on JSR.
for await (
	const entry of walk(ACTOR_BRIDGE_TYPES_PATH, {
		exts: [".ts"],
		includeDirs: false,
	})
) {
	const content = await Deno.readTextFile(entry.path);
	const newContent = content.replace(
		/["']ext:rivet_[^\/]+\/([^\.]*)\.js["']/g,
		'"./$1.d.ts"',
	);
	await Deno.writeTextFile(
		entry.path,
		"// DO NOT MODIFY\n//\n// Generated from sdks/actors-bridge/\n\n" +
			newContent,
	);
}

// Copy types to core repo
copy(ACTOR_BRIDGE_TYPES_PATH, ACTOR_CORE_TYPES_PATH, { recursive: true });
