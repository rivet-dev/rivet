#!/usr/bin/env -S deno run -A

import $ from "dax";
import { exists, walk } from "@std/fs";
import { resolve } from "@std/path";

const actorBridgePath = resolve(
	import.meta.dirname,
	"../../sdks/actors-bridge/",
);
const actorBridgeTypesPath = resolve(actorBridgePath, "types");

// Compile JS bridge
await $`npx -p typescript@5.7.2 tsc -p tsconfig.bridge.json`
	.cwd(actorBridgePath);

// Add header to JS bridge
for await (
	const entry of walk(resolve(import.meta.dirname, "../../packages/infra/client/isolate-v8-runner/js/"), {
		exts: [".js"],
		includeDirs: false,
	})
) {
	const content = await Deno.readTextFile(entry.path);
	await Deno.writeTextFile(
		entry.path,
		"// DO NOT MODIFY\n//\n// Generated with scripts/pegboard/compile_bridge.ts\n\n" +
			content,
	);
}

// Clean types
if (await exists(actorBridgeTypesPath, { directory: true })) {
	await Deno.remove(actorBridgeTypesPath, { recursive: true });
}

// Compile TypeScript types
await $`npx -p typescript@5.7.2 tsc -p tsconfig.types.json`
	.cwd(actorBridgePath);

// Replace imports from `ext:*` to `./*.d.ts`. This is required for this
// package to be usable on JSR.
for await (
	const entry of walk(actorBridgeTypesPath, {
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
