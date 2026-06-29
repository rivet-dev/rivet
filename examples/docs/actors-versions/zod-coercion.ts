import { actor, setup } from "rivetkit";
import { z } from "zod";

const stateSchema = z.object({
	count: z.number().default(0),
	label: z.string().default("default"), // Added in v2
});

type State = z.infer<typeof stateSchema>;

const myActor = actor({
	state: { count: 0, label: "default" } as State,
	onWake: (c) => {
		Object.assign(c.state, stateSchema.parse(c.state));
	},
	actions: {
		getLabel: (c) => c.state.label,
	},
});

const registry2 = setup({ use: { myActor } });
registry2.start();
