import type { Client } from "@/client/client";
import type { ManagerDriver } from "@/manager/driver";
import type { RegistryConfig } from "@/registry/config";
import type { DriverConfig, RunnerConfig } from "@/registry/run-config";
import { RemoteManagerDriver } from "@/remote-manager-driver/mod";
import { EngineActorDriver } from "./actor-driver";

export { EngineActorDriver } from "./actor-driver";
export {
	type EngineConfig as Config,
	type EngineConfigInput as InputConfig,
	EngingConfigSchema as ConfigSchema,
} from "./config";

export function createEngineDriver(): DriverConfig {
	return {
		name: "engine",
		manager: (_registryConfig, runConfig) => {
			return new RemoteManagerDriver(runConfig);
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
	};
}
