import * as fs from "node:fs/promises";
import { resolve } from "node:path";
import { ClientConfigSchema } from "@/client/config";
import { createFileSystemOrMemoryDriver } from "@/drivers/file-system/mod";
import type { ManagerDriver } from "@/manager/driver";
import { createManagerRouter } from "@/manager/router";
import {
	createClientWithDriver,
	type RegistryConfig,
	RegistryConfigSchema,
	setup,
} from "@/mod";
import { type RunnerConfig, RunnerConfigSchema } from "@/registry/run-config";
import { VERSION } from "@/utils";

async function main() {
	const registryConfig: RegistryConfig = RegistryConfigSchema.parse({
		use: {},
	});
	const registry = setup(registryConfig);

	const driverConfig: RunnerConfig = RunnerConfigSchema.parse({
		driver: await createFileSystemOrMemoryDriver(false),
		getUpgradeWebSocket: () => () => unimplemented(),
		inspector: {
			enabled: false,
		},
	});

	const managerDriver: ManagerDriver = {
		getForId: unimplemented,
		getWithKey: unimplemented,
		getOrCreateWithKey: unimplemented,
		createActor: unimplemented,
		sendRequest: unimplemented,
		openWebSocket: unimplemented,
		proxyRequest: unimplemented,
		proxyWebSocket: unimplemented,
		displayInformation: unimplemented,
		getOrCreateInspectorAccessToken: unimplemented,
	};

	const client = createClientWithDriver(
		managerDriver,
		ClientConfigSchema.parse({}),
	);

	const { openapi } = createManagerRouter(
		registryConfig,
		driverConfig,
		managerDriver,
		driverConfig.driver!,
		client,
	);

	const openApiDoc = openapi.getOpenAPIDocument({
		openapi: "3.0.0",
		info: {
			version: VERSION,
			title: "RivetKit API",
		},
	});

	const outputPath = resolve(
		import.meta.dirname,
		"..",
		"..",
		"..",
		"..",
		"rivetkit-openapi",
		"openapi.json",
	);
	await fs.writeFile(outputPath, JSON.stringify(openApiDoc, null, 2));
	console.log("Dumped OpenAPI to", outputPath);
}

function unimplemented(): never {
	throw new Error("UNIMPLEMENTED");
}

main();
