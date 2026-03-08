import { readdirSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import type { AnyActorDefinition } from "@/actor/definition";
import { dynamicActor } from "rivetkit/dynamic";

const FIXTURE_DIR = path.dirname(fileURLToPath(import.meta.url));
const PACKAGE_ROOT = path.resolve(FIXTURE_DIR, "..", "..");
const ACTOR_FIXTURE_DIR = path.join(FIXTURE_DIR, "actors");
const TS_CONFIG_PATH = path.join(PACKAGE_ROOT, "tsconfig.json");

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

async function importActorDefinition(filePath: string): Promise<AnyActorDefinition> {
	const moduleSpecifier = pathToFileURL(filePath).href;
	const module = (await import(moduleSpecifier)) as {
		default?: AnyActorDefinition;
	};

	if (!module.default) {
		throw new Error(
			`driver test actor fixture is missing a default export: ${filePath}`,
		);
	}

	return module.default;
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
			if (typeof esbuild.build !== "function") {
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
			external: ["rivetkit", "rivetkit/*", "@rivetkit/*"],
			logLevel: "silent",
		});

		const outputFile = result.outputFiles.find((file) =>
			file.path.endsWith(".js"),
		);
		if (!outputFile) {
			throw new Error(`failed to bundle dynamic actor source for ${filePath}`);
		}

		return outputFile.text;
	})();

	bundledSourceCache.set(filePath, pendingBundle);
	return await pendingBundle;
}

export async function loadStaticActors(): Promise<
	Record<string, AnyActorDefinition>
> {
	const actors: Record<string, AnyActorDefinition> = {};
	for (const actorFixturePath of listActorFixtureFiles()) {
		actors[actorNameFromFilePath(actorFixturePath)] =
			await importActorDefinition(actorFixturePath);
	}
	return actors;
}

export function loadDynamicActors(): Record<string, DynamicActorDefinition> {
	const actors: Record<string, DynamicActorDefinition> = {};
	for (const actorFixturePath of listActorFixtureFiles()) {
		actors[actorNameFromFilePath(actorFixturePath)] = dynamicActor({
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
