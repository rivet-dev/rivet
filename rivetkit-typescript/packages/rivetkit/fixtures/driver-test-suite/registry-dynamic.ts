import { readdirSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import type { AnyActorDefinition } from "@/actor/definition";
import { setup } from "rivetkit";
import { dynamicActor } from "rivetkit/dynamic";
import type { registry as DriverTestRegistryType } from "./registry-static";
import { registry as staticRegistry } from "./registry-static";

// This file reconstructs the driver fixture registry from per-actor wrappers.
// It exists to verify that the dynamic actor format behaves like the static registry.
const FIXTURE_DIR = path.dirname(fileURLToPath(import.meta.url));
const PACKAGE_ROOT = path.resolve(FIXTURE_DIR, "..", "..");
const ACTOR_FIXTURE_DIR = path.join(FIXTURE_DIR, "actors");
const TS_CONFIG_PATH = path.join(PACKAGE_ROOT, "tsconfig.json");
const RIVETKIT_SOURCE_ALIAS = {
	rivetkit: path.join(PACKAGE_ROOT, "src/mod.ts"),
	"rivetkit/agent-os": path.join(PACKAGE_ROOT, "src/agent-os/index.ts"),
	"rivetkit/db": path.join(PACKAGE_ROOT, "src/db/mod.ts"),
	"rivetkit/db/drizzle": path.join(
		PACKAGE_ROOT,
		"src/db/drizzle/mod.ts",
	),
	"rivetkit/dynamic": path.join(PACKAGE_ROOT, "src/dynamic/mod.ts"),
	"rivetkit/errors": path.join(PACKAGE_ROOT, "src/actor/errors.ts"),
	"rivetkit/sandbox": path.join(PACKAGE_ROOT, "src/sandbox/index.ts"),
	"rivetkit/sandbox/docker": path.join(
		PACKAGE_ROOT,
		"src/sandbox/providers/docker.ts",
	),
	"rivetkit/utils": path.join(PACKAGE_ROOT, "src/utils.ts"),
} as const;
const DYNAMIC_REGISTRY_STATIC_ACTOR_NAMES = new Set([
	"dockerSandboxActor",
	"dockerSandboxControlActor",
]);

type DynamicActorDefinition = ReturnType<typeof dynamicActor>;

interface EsbuildOutputFile {
	path: string;
	text: string;
}

interface EsbuildBuildResult {
	outputFiles: EsbuildOutputFile[];
}

interface EsbuildModule {
	build(options: Record<string, unknown>): Promise<EsbuildBuildResult>;
}

let esbuildModulePromise: Promise<EsbuildModule> | undefined;
const bundledSourceCache = new Map<string, Promise<string>>();

function listActorFixtureFiles(): string[] {
	const entries = readdirSync(ACTOR_FIXTURE_DIR, {
		withFileTypes: true,
	});

	return entries
		.filter((entry) => entry.isFile() && entry.name.endsWith(".ts"))
		.map((entry) => path.join(ACTOR_FIXTURE_DIR, entry.name))
		.sort();
}

function actorNameFromFilePath(filePath: string): string {
	return path.basename(filePath, ".ts");
}

async function loadEsbuildModule(): Promise<EsbuildModule> {
	if (!esbuildModulePromise) {
		esbuildModulePromise = (async () => {
			const runtimeRequire = createRequire(import.meta.url);
			const tsupEntryPath = runtimeRequire.resolve("tsup");
			const tsupRequire = createRequire(tsupEntryPath);
			const esbuildEntryPath = tsupRequire.resolve("esbuild");
			const esbuildModule = (await import(
				pathToFileURL(esbuildEntryPath).href
			)) as EsbuildModule & {
				default?: EsbuildModule;
			};
			const esbuild =
				typeof esbuildModule.build === "function"
					? esbuildModule
					: esbuildModule.default;
			if (!esbuild || typeof esbuild.build !== "function") {
				throw new Error("failed to load esbuild build function");
			}
			return esbuild;
		})();
	}

	return esbuildModulePromise;
}

async function bundleActorFixture(filePath: string): Promise<string> {
	const cached = bundledSourceCache.get(filePath);
	if (cached) {
		return await cached;
	}

	const pendingBundle = (async () => {
		const esbuild = await loadEsbuildModule();
		const result = await esbuild.build({
			absWorkingDir: PACKAGE_ROOT,
			entryPoints: [filePath],
			outfile: "driver-test-actor-bundle.js",
			bundle: true,
			write: false,
			platform: "node",
			format: "esm",
			target: "node22",
			tsconfig: TS_CONFIG_PATH,
			alias: RIVETKIT_SOURCE_ALIAS,
			external: [
				"@rivetkit/*",
				"dockerode",
				"sandbox-agent",
				"sandbox-agent/*",
			],
			logLevel: "silent",
		});

		const outputFile = result.outputFiles.find((file) =>
			file.path.endsWith(".js"),
		);
		if (!outputFile) {
			throw new Error(
				`failed to bundle dynamic actor source for ${filePath}`,
			);
		}

		return outputFile.text;
	})();

	bundledSourceCache.set(filePath, pendingBundle);
	return await pendingBundle;
}

function loadDynamicActors(): Record<string, DynamicActorDefinition> {
	const actors: Record<string, DynamicActorDefinition> = {};
	const staticDefinitions = staticRegistry.config.use as Record<
		string,
		AnyActorDefinition
	>;

	for (const actorFixturePath of listActorFixtureFiles()) {
		const actorName = actorNameFromFilePath(actorFixturePath);
		const staticDefinition = staticDefinitions[actorName];
		if (!staticDefinition) {
			throw new Error(
				`missing static actor definition for dynamic fixture ${actorName}`,
			);
		}
		if (DYNAMIC_REGISTRY_STATIC_ACTOR_NAMES.has(actorName)) {
			actors[actorName] = staticDefinition as DynamicActorDefinition;
			continue;
		}
		actors[actorName] = dynamicActor({
			options: staticDefinition.config.options,
			load: async () => {
				return {
					source: await bundleActorFixture(actorFixturePath),
					sourceFormat: "esm-js" as const,
				};
			},
		});
	}

	return actors;
}

export const registry = setup({
	use: loadDynamicActors(),
}) as unknown as typeof DriverTestRegistryType;
