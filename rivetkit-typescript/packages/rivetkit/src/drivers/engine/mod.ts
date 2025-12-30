import type { Client } from "@/client/client";
import { convertBaseConfigToClientConfig } from "@/client/config";
import type { ManagerDriver } from "@/manager/driver";
import type { RegistryConfig } from "@/registry/config/registry";
import { RemoteManagerDriver } from "@/remote-manager-driver/mod";
import { EngineActorDriver } from "./actor-driver";
import { BaseConfig, DriverConfig } from "@/registry/config/base";
import { RunnerConfig } from "@/registry/config/runner";

export { EngineActorDriver } from "./actor-driver";
export {
	type EngineConfig as Config,
	type EngineConfigInput as InputConfig,
	EngingConfigSchema as ConfigSchema,
} from "./config";

export function createEngineDriver(): DriverConfig {
	return {
		name: "engine",
		displayName: "Engine",
		manager: (_registryConfig: RegistryConfig, runConfig: BaseConfig) => {
			const clientConfig = convertBaseConfigToClientConfig(runConfig);
			return new RemoteManagerDriver(clientConfig);
		},
		actor: (
			registryConfig: RegistryConfig,
			runConfig: RunnerConfig,
			managerDriver: ManagerDriver,
			inlineClient: Client<any>,
		) => {
			return new EngineActorDriver(
				registryConfig,
				runConfig,
				managerDriver,
				inlineClient,
			);
		},
		autoStartActorDriver: true,
	};
}
