import { existsSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { getEnginePath } from "@rivetkit/engine-cli";
import {
	type ActorFactoryHandle,
	actor,
	type CoreRuntime,
	setup,
} from "../../src/mod";
import { buildNativeRegistry } from "../../src/registry/native";

const fixtureDir = dirname(fileURLToPath(import.meta.url));
const repoEngineBinary = resolve(
	fixtureDir,
	"../../../../../target/debug/rivet-engine",
);
const pluginPath = process.env.RIVETKIT_TEST_NATIVE_PLUGIN_PATH;

if (!pluginPath) {
	throw new Error("RIVETKIT_TEST_NATIVE_PLUGIN_PATH is required");
}

function resolveEngineBinaryPath(): string {
	if (existsSync(repoEngineBinary)) {
		return repoEngineBinary;
	}

	return getEnginePath();
}

const nativePluginActor = actor({
	actions: {},
});
nativePluginActor.nativeFactoryBuilder = (
	runtime: CoreRuntime,
): ActorFactoryHandle => {
	if (!runtime.createNativePluginFactory) {
		throw new Error("native plugin factories require the NAPI runtime");
	}

	return runtime.createNativePluginFactory({
		pluginPath,
		configJson: process.env.RIVETKIT_TEST_NATIVE_PLUGIN_CONFIG_JSON ?? "{}",
		sidecarPath: process.env.RIVETKIT_TEST_NATIVE_PLUGIN_SIDECAR_PATH ?? "",
	});
};

const registry = setup({
	use: {
		nativePluginActor,
	},
	endpoint: process.env.RIVETKIT_TEST_ENDPOINT ?? "http://127.0.0.1:6642",
	namespace: process.env.RIVET_NAMESPACE ?? "default",
	token: process.env.RIVET_TOKEN ?? "dev",
	envoy: {
		poolName: process.env.RIVETKIT_TEST_POOL_NAME ?? "default",
	},
});

const { registry: nativeRegistry, serveConfig } = await buildNativeRegistry(
	registry.parseConfig(),
);
serveConfig.engineBinaryPath = resolveEngineBinaryPath();

await nativeRegistry.serve(serveConfig);
