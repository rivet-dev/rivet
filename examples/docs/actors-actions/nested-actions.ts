import { actor } from "rivetkit";

const directory = actor({
	state: { names: [] as string[] },
	actions: {
		// Actions can now be nested
		users: {
			add: (c, name: string) => {
				c.state.names.push(name);
			},
			list: (c) => c.state.names,
		},
	},
});
