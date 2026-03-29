import { agentOs } from "rivetkit/agent-os";
import { setup } from "rivetkit";
import { hostTool, toolKit } from "@rivet-dev/agent-os-core";
import common from "@rivet-dev/agent-os-common";
import { z } from "zod";

const weatherToolkit = toolKit({
	name: "weather",
	description: "Look up weather information for cities.",
	tools: {
		get: hostTool({
			description: "Get the current weather for a city.",
			inputSchema: z.object({
				city: z.string().describe("City name (e.g. 'London')."),
			}),
			execute: async ({ city }) => ({
				city,
				temperature: 18,
				conditions: "partly cloudy",
				humidity: 65,
			}),
			examples: [
				{ description: "Get London weather", input: { city: "London" } },
			],
		}),
	},
});

const calcToolkit = toolKit({
	name: "calc",
	description: "Simple calculator operations.",
	tools: {
		add: hostTool({
			description: "Add two numbers.",
			inputSchema: z.object({ a: z.number(), b: z.number() }),
			execute: ({ a, b }) => ({ result: a + b }),
		}),
	},
});

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const vm = agentOs({
	options: {
		software: [common],
		toolKits: [weatherToolkit, calcToolkit],
	},
}) as any;

export const registry = setup({ use: { vm } });
registry.start();
