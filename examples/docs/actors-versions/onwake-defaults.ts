import { actor, setup } from "rivetkit";

const myActor = actor({
	state: { count: 0, label: "" },
	onWake: (c) => {
		// Added in v2. Old actors won't have this field.
		c.state.label ??= "default";
	},
	actions: {
		getLabel: (c) => c.state.label,
	},
});

const registry = setup({ use: { myActor } });
registry.start();
