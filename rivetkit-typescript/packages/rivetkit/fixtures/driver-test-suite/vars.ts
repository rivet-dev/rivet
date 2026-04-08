import { actor } from "rivetkit";

// Actor with static vars
export const staticVarActor = actor({
	state: { value: 0 },
	connState: { hello: "world" },
	vars: { counter: 42, name: "test-actor" },
	actions: {
		getVars: (c) => {
			return c.vars;
		},
		getName: (c) => {
			return c.vars.name;
		},
	},
});

// Actor with nested vars
export const nestedVarActor = actor({
	state: { value: 0 },
	connState: { hello: "world" },
	vars: {
		counter: 42,
		nested: {
			value: "original",
			array: [1, 2, 3],
			obj: { key: "value" },
		},
	},
	actions: {
		getVars: (c) => {
			return c.vars;
		},
		modifyNested: (c) => {
			// Attempt to modify the nested object
			c.vars.nested.value = "modified";
			c.vars.nested.array.push(4);
			c.vars.nested.obj.key = "new-value";
			return c.vars;
		},
	},
});

// Actor with dynamic vars
export const dynamicVarActor = actor({
	state: { value: 0 },
	connState: { hello: "world" },
	createVars: () => {
		return {
			random: Math.random(),
			computed: `Actor-${Math.floor(Math.random() * 1000)}`,
		};
	},
	actions: {
		getVars: (c) => {
			return c.vars;
		},
	},
});

// Actor with unique vars per instance
export const uniqueVarActor = actor({
	state: { value: 0 },
	connState: { hello: "world" },
	createVars: () => {
		return {
			id: Math.floor(Math.random() * 1000000),
		};
	},
	actions: {
		getVars: (c) => {
			return c.vars;
		},
	},
});

// Actor that uses driver context
export const driverCtxActor = actor({
	state: { value: 0 },
	connState: { hello: "world" },
	createVars: (c, driverCtx: any) => {
		return {
			hasDriverCtx: Boolean(driverCtx?.isTest),
		};
	},
	actions: {
		getVars: (c) => {
			return c.vars;
		},
	},
});
