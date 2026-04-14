// @ts-nocheck
import { actor, queue } from "rivetkit";
import type { registry } from "./registry-static";

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
	createVars: () => ({ ephemeral: "fresh" }),
	queues: {
		values: queue<number>(),
	},
	onWake: (c) => {
		// Store the actor key so we can reference it in onDestroy
		c.state.key = c.key.join("/");
	},
	onRequest: (c, request) => {
		const url = new URL(request.url);
		if (url.pathname === "/state") {
			return new Response(
				JSON.stringify({
					key: c.state.key,
					value: c.state.value,
				}),
				{
					headers: {
						"content-type": "application/json",
					},
				},
			);
		}

		return new Response("Not Found", { status: 404 });
	},
	onWebSocket: (c, websocket) => {
		websocket.send(
			JSON.stringify({
				type: "welcome",
				key: c.state.key,
				value: c.state.value,
			}),
		);
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
		setEphemeral: (c, value: string) => {
			c.vars.ephemeral = value;
			return c.vars.ephemeral;
		},
		getEphemeral: (c) => {
			return c.vars.ephemeral;
		},
		receiveValue: async (c) => {
			const message = await c.queue.next({
				names: ["values"],
				timeout: 0,
			});
			return message?.body ?? null;
		},
		destroy: (c) => {
			c.destroy();
		},
	},
});
