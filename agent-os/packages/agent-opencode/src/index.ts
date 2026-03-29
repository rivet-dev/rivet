import { defineSoftware } from "@rivet-dev/agent-os-core";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const packageDir = resolve(__dirname, "..");

const opencode = defineSoftware({
	name: "opencode",
	type: "agent" as const,
	packageDir,
	requires: ["opencode-ai"],
	agent: {
		id: "opencode",
		// OpenCode speaks ACP natively. No separate adapter wrapper needed.
		// NOTE: OpenCode is a native binary, not Node.js. It cannot currently
		// run inside the secure-exec VM (kernel only supports JS/WASM commands).
		acpAdapter: "opencode-ai",
		agentPackage: "opencode-ai",
		prepareInstructions: async (kernel, _cwd, additionalInstructions, opts) => {
			const contextPaths = opts?.skipBase
				? []
				: [
						".github/copilot-instructions.md",
						".cursorrules",
						".cursor/rules/",
						"CLAUDE.md",
						"CLAUDE.local.md",
						"opencode.md",
						"opencode.local.md",
						"OpenCode.md",
						"OpenCode.local.md",
						"OPENCODE.md",
						"OPENCODE.local.md",
						"/etc/agentos/instructions.md",
					];
			if (additionalInstructions) {
				const additionalPath = "/tmp/agentos-additional-instructions.md";
				await kernel.writeFile(additionalPath, additionalInstructions);
				contextPaths.push(additionalPath);
			}
			if (opts?.toolReference) {
				const toolRefPath = "/tmp/agentos-tool-reference.md";
				await kernel.writeFile(toolRefPath, opts.toolReference);
				contextPaths.push(toolRefPath);
			}
			if (contextPaths.length === 0) return {};
			return {
				env: { OPENCODE_CONTEXTPATHS: JSON.stringify(contextPaths) },
			};
		},
	},
});

export default opencode;
