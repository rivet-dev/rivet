import { actor } from "rivetkit";

export const onStateChangeActor = actor({
	state: {
		value: 0,
		blob: [] as number[],
	},
	vars: {
		changeCount: 0,
	},
	actions: {
		// Action that modifies state - should trigger onStateChange
		setValue: (c, newValue: number) => {
			c.state.value = newValue;
			return c.state.value;
		},
		// Action that modifies state many times in one synchronous burst. The
		// resulting onStateChange and save are coalesced to once per tick.
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
			return c.vars.changeCount;
		},
		// Reset change counter for testing
		resetChangeCount: (c) => {
			c.vars.changeCount = 0;
		},
		// Returns true if reading `c.state` twice yields the same object. A
		// stable identity means the write-through proxy is memoized rather than
		// rebuilt on every access.
		stateIdentityStable: (c) => {
			return c.state === c.state;
		},
		// Seed a large state payload so a mutation burst has to re-serialize a
		// substantial object if serialization is not coalesced.
		seedLarge: (c, size: number) => {
			c.state.blob = Array.from({ length: size }, (_, i) => i);
			return c.state.blob.length;
		},
		// Run a synchronous burst of mutations and report how long the burst
		// blocked the event loop. With per-mutation serialization this scales
		// with state size times the mutation count.
		churn: (c, times: number) => {
			const start = performance.now();
			for (let i = 0; i < times; i++) {
				c.state.value++;
			}
			return performance.now() - start;
		},
	},
	// Track onStateChange calls
	onStateChange: (c) => {
		c.vars.changeCount++;
	},
});
