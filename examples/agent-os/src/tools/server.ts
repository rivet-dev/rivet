import { agentOs } from "rivetkit/agent-os";
import { setup } from "rivetkit";
import { hostTool, toolKit } from "@rivet-dev/agent-os-core";
import common from "@rivet-dev/agent-os-common";
import pi from "@rivet-dev/agent-os-pi";
import { z } from "zod";

const weatherToolkit = toolKit({
	name: "weather",
	description: "Look up weather information for cities.",
	tools: {
		forecast: hostTool({
			description: "Get the weather forecast for a city.",
			inputSchema: z.object({
				city: z.string().describe("City name (e.g. 'Paris')."),
				days: z.number().optional().describe("Number of days"),
			}),
			execute: async ({ city, days }) => ({
				city,
				days: days ?? 3,
				temperature: 22,
				conditions: "sunny",
			}),
			examples: [
				{ description: "3-day forecast for Paris", input: { city: "Paris", days: 3 } },
			],
		}),
	},
});

// Tools are exposed as CLI commands inside the VM:
//
//   # List all available toolkits
//   agentos list-tools
//
//   # Get help for a tool
//   agentos-weather forecast --help
//
//   # Call a tool with flags
//   agentos-weather forecast --city Paris --days 3
//
//   # Call with inline JSON
//   agentos-weather forecast --json '{"city":"Paris","days":3}'

const vm = agentOs({
	options: {
		software: [common, pi],
		toolKits: [weatherToolkit],
	},
});

export const registry = setup({ use: { vm } });
registry.start();
