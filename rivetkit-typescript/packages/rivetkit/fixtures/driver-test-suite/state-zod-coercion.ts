import { actor } from "rivetkit";
import { z } from "zod";

const stateSchema = z.object({
	count: z.number().default(0),
	label: z.string().default("default"),
});

type State = z.infer<typeof stateSchema>;

export const stateZodCoercionActor = actor({
	state: { count: 0, label: "default" } as State,
	onWake: (c) => {
		Object.assign(c.state, stateSchema.parse(c.state));
	},
	actions: {
		getState: (c) => ({ count: c.state.count, label: c.state.label }),
		setCount: (c, count: number) => {
			c.state.count = count;
		},
		setLabel: (c, label: string) => {
			c.state.label = label;
		},
		triggerSleep: (c) => {
			c.sleep();
		},
	},
	options: {
		sleepTimeout: 100,
	},
});
