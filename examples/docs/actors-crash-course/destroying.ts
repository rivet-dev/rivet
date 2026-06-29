import { actor } from "rivetkit";

const userAccount = actor({
	state: { email: "", name: "" },
	onDestroy: (c) => {
		console.log(`Account ${c.state.email} deleted`);
	},
	actions: {
		deleteAccount: (c) => {
			c.destroy();
		},
	},
});
