export interface ActorConfig {
	rpc: RpcConfig;
}

export interface RpcConfig {
	timeout: number;
}

export const DEFAULT_ACTOR_CONFIG: ActorConfig = {
	rpc: {
		timeout: 5000,
	},
};

export function mergeActorConfig(
	partialConfig?: Partial<ActorConfig>,
): ActorConfig {
	return {
		rpc: {
			timeout: partialConfig?.rpc?.timeout ?? DEFAULT_ACTOR_CONFIG.rpc.timeout,
		},
	};
}
