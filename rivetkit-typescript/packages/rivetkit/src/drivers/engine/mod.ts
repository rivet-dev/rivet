import type { Client } from "@/client/client";
import { convertRegistryConfigToClientConfig } from "@/client/config";
import type { ManagerDriver } from "@/manager/driver";
import { RemoteManagerDriver } from "@/remote-manager-driver/mod";
import { EngineActorDriver } from "./actor-driver";
import { RegistryConfig, DriverConfig } from "@/registry/config";

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
		manager: (config: RegistryConfig) => {
			const clientConfig = convertRegistryConfigToClientConfig(config);
			return new RemoteManagerDriver(clientConfig);
		},
		actor: (
			config: RegistryConfig,
			managerDriver: ManagerDriver,
			inlineClient: Client<any>,
		) => {
			return new EngineActorDriver(
				config,
				managerDriver,
				inlineClient,
			);
		},
		autoStartActorDriver: true,
	};
}
