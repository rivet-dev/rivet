import type { RecursivePartial } from "./utils";

export interface ActorConfig {
	protocol: {
		maxConnectionParametersSize: number;
		maxIncomingMessageSize: number;
	};
	state: StateConfig;
	rpc: RpcConfig;
}

export interface StateConfig {
	saveInterval: number;
}

export interface RpcConfig {
	timeout: number;
}

export const DEFAULT_ACTOR_CONFIG: ActorConfig = {
	protocol: {
		// This goes in the URL so the default needs to be short
		maxConnectionParametersSize: 8_192,
		maxIncomingMessageSize: 65_536,
	},
	state: {
		saveInterval: 1000,
	},
	rpc: {
		timeout: 5000,
	},
};

export function mergeActorConfig(
	partialConfig?: RecursivePartial<ActorConfig>,
): ActorConfig {
	return {
		protocol: {
			maxConnectionParametersSize:
				partialConfig?.protocol?.maxConnectionParametersSize ??
				DEFAULT_ACTOR_CONFIG.protocol.maxConnectionParametersSize,
			maxIncomingMessageSize:
				partialConfig?.protocol?.maxIncomingMessageSize ??
				DEFAULT_ACTOR_CONFIG.protocol.maxIncomingMessageSize,
		},
		state: {
			saveInterval:
				partialConfig?.state?.saveInterval ??
				DEFAULT_ACTOR_CONFIG.state.saveInterval,
		},
		rpc: {
			timeout:
				partialConfig?.rpc?.timeout ?? DEFAULT_ACTOR_CONFIG.rpc.timeout,
		},
	};
}
