import { actor } from "rivetkit";

export const onStateChangeActor = actor({
	state: {
		value: 0,
		changeCount: 0,
	},
	actions: {
		// Action that modifies state - should trigger onStateChange
		setValue: (c, newValue: number) => {
			c.state.value = newValue;
			return c.state.value;
		},
		// Action that modifies state multiple times - should trigger onStateChange for each change
		incrementMultiple: (c, times: number) => {
			for (let i = 0; i < times; i++) {
				c.state.value++;
			}
			return c.state.value;
		},
		// Action that doesn't modify state - should NOT trigger onStateChange
		getValue: (c) => {
			return c.state.value;
		},
		// Action that reads and returns without modifying - should NOT trigger onStateChange
		getDoubled: (c) => {
			const doubled = c.state.value * 2;
			return doubled;
		},
		// Get the count of how many times onStateChange was called
		getChangeCount: (c) => {
			return c.state.changeCount;
		},
		// Reset change counter for testing
		resetChangeCount: (c) => {
			c.state.changeCount = 0;
		},
	},
	// Track onStateChange calls
	onStateChange: (c) => {
		c.state.changeCount++;
	},
});
