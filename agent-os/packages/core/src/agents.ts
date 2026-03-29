// Agent configurations for ACP-compatible coding agents

import type { Kernel } from "@secure-exec/core";

const INSTRUCTIONS_PATH = "/etc/agentos/instructions.md";

/**
 * Read OS instructions from /etc/agentos/instructions.md inside the VM,
 * optionally appending session-level additional instructions and tool reference.
 * When skipBase is true, the OS base file is not read (used for tool-docs-only injection).
 */
async function readVmInstructions(
	kernel: Kernel,
	additionalInstructions?: string,
	toolReference?: string,
	skipBase?: boolean,
): Promise<string> {
	const parts: string[] = [];
	if (!skipBase) {
		const data = await kernel.readFile(INSTRUCTIONS_PATH);
		parts.push(new TextDecoder().decode(data));
	}
	if (additionalInstructions) parts.push(additionalInstructions);
	if (toolReference) parts.push(toolReference);
	// Append a horizontal rule so agents can distinguish the injected
	// system prompt from whatever the host appends after it.
	parts.push("---");
	return parts.join("\n\n");
}

/** Options passed alongside additionalInstructions in prepareInstructions. */
export interface PrepareInstructionsOptions {
	/** Auto-generated tool reference markdown to append to the prompt. */
	toolReference?: string;
	/** When true, skip reading the base OS instructions file. */
	skipBase?: boolean;
}

export interface AgentConfig {
	/** npm package name for the ACP adapter (spawned inside the VM) */
	acpAdapter: string;
	/** npm package name for the underlying agent */
	agentPackage: string;
	/**
	 * Default env vars to pass when spawning the adapter. These are merged
	 * UNDER prepareInstructions env and user env (lowest priority).
	 * Typically set by package descriptors for computed paths (e.g. PI_ACP_PI_COMMAND).
	 */
	defaultEnv?: Record<string, string>;
	/**
	 * Prepare agent-specific spawn overrides for OS instruction injection.
	 * Reads /etc/agentos/instructions.md from the VM filesystem (written at boot)
	 * and returns extra CLI args and env vars to merge into the spawn call.
	 *
	 * IMPORTANT: Must extend (not replace) the user's existing config.
	 * User-provided env vars and args always take priority — callers merge as:
	 *   env: { ...prepareInstructions().env, ...userEnv }
	 */
	prepareInstructions?(
		kernel: Kernel,
		cwd: string,
		additionalInstructions?: string,
		options?: PrepareInstructionsOptions,
	): Promise<{ args?: string[]; env?: Record<string, string> }>;
}

export type AgentType = "pi" | "opencode";

export const AGENT_CONFIGS: Record<AgentType, AgentConfig> = {
	pi: {
		acpAdapter: "pi-acp",
		agentPackage: "@mariozechner/pi-coding-agent",
		// OS instructions injection: reads /etc/agentos/instructions.md from VM,
		// passes via --append-system-prompt. User's AGENTS.md/CLAUDE.md at cwd
		// still loads via PI's directory walk. Zero filesystem writes.
		prepareInstructions: async (
			kernel,
			_cwd,
			additionalInstructions,
			opts,
		) => {
			const instructions = await readVmInstructions(
				kernel,
				additionalInstructions,
				opts?.toolReference,
				opts?.skipBase,
			);
			if (!instructions) return {};
			return { args: ["--append-system-prompt", instructions] };
		},
	},
	opencode: {
		// OpenCode speaks ACP natively — no separate adapter wrapper needed.
		// NOTE: OpenCode is a native binary, not Node.js. It cannot currently
		// run inside the secure-exec VM (kernel only supports JS/WASM commands).
		acpAdapter: "opencode-ai",
		agentPackage: "opencode-ai",
		// OS instructions injection: OPENCODE_CONTEXTPATHS env var with absolute
		// path to /etc/agentos/instructions.md. No cwd file writes needed — the
		// file is already on disk from VM boot. /etc/agentos/ is read-only so we
		// never write there. If session-level additional instructions are provided,
		// they are written to /tmp/ and the path is added to OPENCODE_CONTEXTPATHS.
		prepareInstructions: async (
			kernel,
			_cwd,
			additionalInstructions,
			opts,
		) => {
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
						INSTRUCTIONS_PATH,
					];
			if (additionalInstructions) {
				const additionalPath =
					"/tmp/agentos-additional-instructions.md";
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
};

// ── Agents not yet in AGENT_CONFIGS ─────────────────────────────────────
//
// Claude Code (@anthropic-ai/claude-code)
//   Cannot run in VM: native ripgrep dep, complex async startup, no TTY.
//   Speaks ACP natively (cli.js, no separate adapter).
//   Injection approach: reads /etc/agentos/instructions.md from VM,
//   passes via --append-system-prompt <text> CLI flag.
//   Zero filesystem writes. User's CLAUDE.md still loads normally.
//   Config when runnable:
//     acpAdapter: "@anthropic-ai/claude-code"
//     agentPackage: "@anthropic-ai/claude-code"
//     prepareInstructions: async (kernel, _cwd, additionalInstructions) => {
//       const instructions = await readVmInstructions(kernel, additionalInstructions);
//       return { args: ["--append-system-prompt", instructions] };
//     }
//
// Codex (@openai/codex)
//   Not yet investigated for VM compatibility (Rust binary).
//   Injection approach: reads /etc/agentos/instructions.md from VM,
//   passes via -c developer_instructions="..." CLI flag.
//   Injected as additive developer role message — does not replace built-in
//   system instructions. User's AGENTS.md still loads normally.
//   Zero filesystem writes.
//   Config when runnable:
//     acpAdapter: "@openai/codex" (or dedicated ACP adapter TBD)
//     agentPackage: "@openai/codex"
//     prepareInstructions: async (kernel, _cwd, additionalInstructions) => {
//       const instructions = await readVmInstructions(kernel, additionalInstructions);
//       return { args: ["-c", `developer_instructions=${instructions}`] };
//     }
