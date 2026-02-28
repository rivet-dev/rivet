import { actor, setup } from "rivetkit";
import { dynamicActor } from "rivetkit/dynamic";

export const DEFAULT_DYNAMIC_ACTOR_SOURCE = `import { actor } from "rivetkit";

export default actor({
	state: {
		count: 0,
	},
	actions: {
		increment: (c, amount = 1) => {
			c.state.count += amount;
			return c.state.count;
		},
		getCount: (c) => c.state.count,
	},
});
`;

const sourceCode = actor({
	state: {
		source: DEFAULT_DYNAMIC_ACTOR_SOURCE,
		revision: 1,
	},
	actions: {
		getSource: (c: any) => ({
			source: c.state.source,
			revision: c.state.revision,
		}),
		setSource: (c: any, source: string) => {
			c.state.source = source;
			c.state.revision += 1;
			return { revision: c.state.revision };
		},
	},
});

const dynamicWorkflow = dynamicActor(async (c: any) => {
	const sourceState = await c
		.client()
		.sourceCode.getOrCreate(["main"])
		.getSource();

	return {
		source: sourceState.source,
		nodeProcess: {
			memoryLimit: 256,
			cpuTimeLimitMs: 10_000,
		},
	};
});

export const registry = setup({
	use: {
		sourceCode,
		dynamicWorkflow,
	},
});

// // ===
//
// import { actor, setup } from "rivetkit";
// import { dynamicActor } from "rivetkit/dynamic";
//
// const dynamicWorkflow = dynamicActor(async (c: any) => {
// 	// Load actor code from external source based on actor key
// 	const source = await fetch(/* ... */);
//
// 	return {
// 		source: sourceState.source,
// 		nodeProcess: {
// 			memoryLimit: 256,
// 			cpuTimeLimitMs: 10_000,
// 		},
// 	};
// });
//
// export const registry = setup({
// 	use: {
// 		dynamicWorkflow
// 	},
// });
