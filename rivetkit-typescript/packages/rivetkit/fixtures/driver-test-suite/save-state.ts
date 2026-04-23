// @ts-nocheck
import { actor } from "rivetkit";
import type { registry } from "./registry-static";

export const saveStateObserver = actor({
	state: { phases: {} as Record<string, string> },
	actions: {
		recordPhase: (c, actorKey: string, phase: string) => {
			c.state.phases[actorKey] = phase;
		},
		getPhase: (c, actorKey: string) => {
			return c.state.phases[actorKey] ?? null;
		},
		reset: (c, actorKey: string) => {
			delete c.state.phases[actorKey];
		},
	},
});

export const saveStateActor = actor({
	state: { value: 0 },
	actions: {
		getValue: (c) => {
			return c.state.value;
		},
		saveImmediateAndBlock: async (c, value: number) => {
			c.state.value = value;
			await c.saveState({ immediate: true });

			const observer = c.client<typeof registry>().saveStateObserver.getOrCreate([
				"observer",
			]);
			await observer.recordPhase(c.key.join("/"), "immediate");

			await new Promise<void>(() => {});
		},
		saveDeferredAndBlock: async (
			c,
			value: number,
			maxWaitMs: number,
		) => {
			c.state.value = value;
			await c.saveState({ maxWait: maxWaitMs });

			const observer = c.client<typeof registry>().saveStateObserver.getOrCreate([
				"observer",
			]);
			await observer.recordPhase(c.key.join("/"), "deferred");

			await new Promise<void>(() => {});
		},
	},
});
