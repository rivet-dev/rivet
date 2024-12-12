export interface ActorConfig {
	state: StateConfig,
	rpc: RpcConfig;
}

export interface StateConfig {
	saveInterval: number;
}

export interface RpcConfig {
	timeout: number;
}

export const DEFAULT_ACTOR_CONFIG: ActorConfig = {
	state: {
		saveInterval: 1000,
	},
	rpc: {
		timeout: 5000,
	},
};

export function mergeActorConfig(
	partialConfig?: Partial<ActorConfig>,
): ActorConfig {
	return {
		state: {
			saveInterval: partialConfig?.state?.saveInterval ?? DEFAULT_ACTOR_CONFIG.state.saveInterval,
		},
		rpc: {
			timeout: partialConfig?.rpc?.timeout ?? DEFAULT_ACTOR_CONFIG.rpc.timeout,
		},
	};
}
