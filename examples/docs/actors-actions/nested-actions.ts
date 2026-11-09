import { actor } from "rivetkit";

const directory = actor({
	vars: { names: [] as string[] },
	actions: {
		// Actions can now be nested
		users: {
			add: (c, name: string) => {
				c.vars.names.push(name);
			},
			list: (c) => c.vars.names,
		},
	},
});
