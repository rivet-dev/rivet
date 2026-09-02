import { event } from "rivetkit";
import { describe, expect, it } from "vitest";
import { pi } from "../src/index.js";

describe("pi actor definition", () => {
	it("preserves user actions, events, state, and options", () => {
		const definition = pi({
			createState: () => ({ count: 0 }),
			events: { customEvent: event<string>() },
			actions: {
				increment: (context: any, amount: number) => {
					context.state.count += amount;
					return context.state.count;
				},
			},
			options: { sleepTimeout: 1234 },
		});

		expect(Object.keys(definition.config.actions ?? {})).toEqual(
			expect.arrayContaining(["increment", "prompt", "getSession"]),
		);
		expect(Object.keys(definition.config.events ?? {})).toEqual(
			expect.arrayContaining(["customEvent", "event"]),
		);
		expect("createState" in definition.config).toBe(true);
		expect(definition.config.options?.sleepTimeout).toBe(1234);
	});

	it("rejects collisions with built-in actions and events", () => {
		expect(() =>
			pi({ actions: { prompt: () => "custom" } }),
		).toThrow("pi() action name is reserved: prompt");
		expect(() => pi({ events: { event: event<string>() } })).toThrow(
			"pi() event name is reserved: event",
		);
	});
});
