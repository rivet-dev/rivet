import { dynamicActor } from "rivetkit/dynamic";

// The dynamic actor loads its code from the codeAgent with the matching key.
export const dynamicRunner = dynamicActor({
	load: async (c: any) => {
		// Extract the coding agent that owns the code
		const codeAgentKey = c.key.slice(0, -1);
		const client = await c.client();
		const state = await client.codeAgent.getOrCreate(codeAgentKey).getState();

		return {
			source: state.code,
			nodeProcess: {
				memoryLimit: 256,
				cpuTimeLimitMs: 10_000,
			},
		};
	},
});
