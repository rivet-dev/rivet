import { actor } from "rivetkit";
import type { registry } from "../../actors.ts";

export const destroyObserver = actor({
	state: { destroyedActors: [] as string[] },
	actions: {
		notifyDestroyed: (c, actorKey: string) => {
			c.state.destroyedActors.push(actorKey);
		},
		wasDestroyed: (c, actorKey: string) => {
			return c.state.destroyedActors.includes(actorKey);
		},
		reset: (c) => {
			c.state.destroyedActors = [];
		},
	},
});

export const destroyActor = actor({
	state: { value: 0, key: "" },
	onWake: (c) => {
		// Store the actor key so we can reference it in onDestroy
		c.state.key = c.key.join("/");
	},
	onDestroy: async (c) => {
		const client = c.client<typeof registry>();
		const observer = client.destroyObserver.getOrCreate(["observer"]);
		await observer.notifyDestroyed(c.state.key);
	},
	actions: {
		setValue: async (c, newValue: number) => {
			c.state.value = newValue;
			await c.saveState({ immediate: true });
			return c.state.value;
		},
		getValue: (c) => {
			return c.state.value;
		},
		destroy: (c) => {
			c.destroy();
		},
	},
});
