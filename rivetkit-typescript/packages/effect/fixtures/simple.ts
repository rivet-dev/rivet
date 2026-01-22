import { actor } from "rivetkit";

// Simple actor - plain functions without Effect wrappers for comparison
export const simple = actor({
	state: {
		value: 0,
	},
	actions: {
		getValue: (c) => {
			return c.state.value;
		},
		setValue: (c, x: number) => {
			c.state.value = x;
			return c.state.value;
		},
	},
});
