import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import type { Registry } from "../../src/registry";
import { buildNativeRegistry } from "../../src/registry/native";

const registryPath = process.env.RIVETKIT_DRIVER_REGISTRY_PATH;
const endpoint = process.env.RIVETKIT_TEST_ENDPOINT;
const token = process.env.RIVET_TOKEN ?? "dev";
const namespace = process.env.RIVET_NAMESPACE ?? "default";
const poolName = process.env.RIVETKIT_TEST_POOL_NAME ?? "default";

if (!registryPath) {
	throw new Error("RIVETKIT_DRIVER_REGISTRY_PATH is required");
}

if (!endpoint) {
	throw new Error("RIVETKIT_TEST_ENDPOINT is required");
}

const { registry } = (await import(
	pathToFileURL(resolve(registryPath)).href
)) as {
	registry: Registry<any>;
};

registry.config.test = { ...registry.config.test, enabled: true };
registry.config.startEngine = false;
registry.config.endpoint = endpoint;
registry.config.token = token;
registry.config.namespace = namespace;
registry.config.envoy = {
	...registry.config.envoy,
	poolName,
};

const { registry: nativeRegistry, serveConfig } = await buildNativeRegistry(
	registry.parseConfig(),
);

await nativeRegistry.serve(serveConfig);
