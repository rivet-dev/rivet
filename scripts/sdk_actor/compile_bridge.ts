#!/usr/bin/env -S deno run -A

import $ from "dax";
import { copy, walk } from "@std/fs";
import { resolve } from "@std/path";

const PROJECT_ROOT = resolve(import.meta.dirname!, "../..");
const ACTOR_SDK_PATH = resolve(PROJECT_ROOT, "sdks/actor");

const ACTOR_BRIDGE_PATH = resolve(ACTOR_SDK_PATH, "bridge");
const ACTOR_BRIDGE_TYPES_PATH = resolve(ACTOR_BRIDGE_PATH, "types");

const ISOLATE_RUNNER_JS_PATH = resolve(
	import.meta.dirname!,
	"../../packages/edge/infra/client/isolate-v8-runner/js/",
);

const ACTOR_CORE_BRIDGE_TYPES_PATH = resolve(
	ACTOR_SDK_PATH,
	"core",
	"src",
	"bridge_types",
);

// Clean folders
await Deno.remove(ACTOR_BRIDGE_TYPES_PATH, { recursive: true }).catch(() => {});
await Deno.remove(ACTOR_CORE_BRIDGE_TYPES_PATH, { recursive: true }).catch(
	() => {},
);
await Deno.remove(ISOLATE_RUNNER_JS_PATH, { recursive: true }).catch(() => {});

// Compile JS bridge
await $`npx -p typescript@5.7.2 tsc -p tsconfig.bridge.json`.cwd(
	ACTOR_BRIDGE_PATH,
);

for await (const entry of walk(ISOLATE_RUNNER_JS_PATH, {
	exts: [".js"],
	includeDirs: false,
})) {
	let content = await Deno.readTextFile(entry.path);

	// Add header to JS bridge
	await Deno.writeTextFile(
		entry.path,
		"// DO NOT MODIFY\n//\n// Generated with scripts/sdk_actor/compile_bridge.ts\n\n" +
			content,
	);
}

// Compile TypeScript types
await $`npx -p typescript@5.7.2 tsc -p tsconfig.types.json`.cwd(
	ACTOR_BRIDGE_PATH,
);

// Copy internal types file, since TypeScript doesn't copy type declarations
await copy(
	resolve(ACTOR_BRIDGE_PATH, "src", "bridge", "types"),
	resolve(ACTOR_BRIDGE_TYPES_PATH, "types"),
);

// Format types. Needs to run in ACTOR_SDK_PATH so it has access to the biome config.
await $`npx -p @biomejs/biome@1.9.4 biome check --write sdks/actor/bridge/types/`.cwd(
	PROJECT_ROOT,
);

for await (const entry of walk(ACTOR_BRIDGE_TYPES_PATH, {
	exts: [".ts"],
	includeDirs: false,
})) {
	let content = await Deno.readTextFile(entry.path);

	// Remove `#private` since it causes errors when coercing types with generated .d.ts.
	content = content.replace(/#private;/g, "");

	// Replace imports from `*.js` to `./*.d.ts`. This is required for the types to
	// be resolved.
	content = content.replace(/["'](\.[^\.]*)\.js["']/g, '"$1.d.ts"');

	// Replace imports from `ext:*` to `./*.d.ts`. This is required for this
	// package to be usable on JSR.
	content = content.replace(
		/["']ext:rivet_[^\/]+\/([^\.]*)\.js["']/g,
		'"./$1.d.ts"',
	);

	await Deno.writeTextFile(
		entry.path,
		"// DO NOT MODIFY\n//\n// Generated from sdks/actor/bridge/\n\n" + content,
	);
}

// Copy types to core repo
await copy(ACTOR_BRIDGE_TYPES_PATH, ACTOR_CORE_BRIDGE_TYPES_PATH);
