import { defineSoftware } from "@rivet-dev/agent-os-core";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const packageDir = resolve(__dirname, "..");

const pi = defineSoftware({
	name: "pi",
	type: "agent" as const,
	packageDir,
	requires: ["pi-acp", "@mariozechner/pi-coding-agent"],
	agent: {
		id: "pi",
		acpAdapter: "pi-acp",
		agentPackage: "@mariozechner/pi-coding-agent",
		env: (ctx) => ({
			PI_ACP_PI_COMMAND: ctx.resolveBin(
				"@mariozechner/pi-coding-agent",
				"pi",
			),
		}),
		prepareInstructions: async (kernel, _cwd, additionalInstructions, opts) => {
			const parts: string[] = [];
			if (!opts?.skipBase) {
				const data = await kernel.readFile("/etc/agentos/instructions.md");
				parts.push(new TextDecoder().decode(data));
			}
			if (additionalInstructions) parts.push(additionalInstructions);
			if (opts?.toolReference) parts.push(opts.toolReference);
			parts.push("---");
			const instructions = parts.join("\n\n");
			if (!instructions) return {};
			return { args: ["--append-system-prompt", instructions] };
		},
	},
});

export default pi;
