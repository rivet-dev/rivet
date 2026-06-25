// Pure generator: maps the onboarding agentOS selections to the install command,
// the server.ts/client.ts the user hands to their coding agent, and a handoff
// prompt that enumerates the selections so the agent scaffolds matching code.
//
// The generated code mirrors the verified in-repo examples:
//   examples/agent-os/src/agent-session/server.ts  (minimal)
//   examples/agent-os/src/sandbox/server.ts         (sandbox on)
// Software/agent packages are default imports passed to `software: [...]`.

import {
	AGENTS,
	DEFAULT_AGENT,
	DEFAULT_SANDBOX_PROVIDER,
	SOFTWARE,
	type SoftwareEntry,
} from "./catalog";

export interface AgentOsSelections {
	agent: string;
	packages: string[];
	sandbox: { enabled: boolean; provider?: string };
}

export interface AgentOsSetup {
	installCommand: string;
	serverCode: string;
	clientCode: string;
	prompt: string;
}

// kebab-case slug -> a valid default-import identifier (e.g. build-essential -> buildEssential).
function importName(slug: string): string {
	return slug.replace(/-([a-z])/g, (_, c: string) => c.toUpperCase());
}

function resolveAgent(slug: string) {
	const found = AGENTS.find((a) => a.slug === slug && a.status === "available");
	// Fall back to the default available agent (Pi) if an unavailable one slips through.
	return found?.package
		? found
		: (AGENTS.find((a) => a.slug === DEFAULT_AGENT) ?? AGENTS[0]);
}

export function buildAgentOsSetup(selections: AgentOsSelections): AgentOsSetup {
	const agent = resolveAgent(selections.agent);
	const agentPackage = agent.package ?? "@rivet-dev/agent-os-pi";
	const agentSymbol = importName(agent.slug);

	const softwareEntries: SoftwareEntry[] = selections.packages
		.map((slug) => SOFTWARE.find((s) => s.slug === slug))
		.filter((s): s is SoftwareEntry => Boolean(s));

	const sandboxEnabled = selections.sandbox.enabled;
	const provider = selections.sandbox.provider ?? DEFAULT_SANDBOX_PROVIDER;

	const installPackages = [
		"rivetkit",
		...softwareEntries.map((s) => s.package),
		agentPackage,
		...(sandboxEnabled
			? ["@rivet-dev/agent-os-sandbox", "sandbox-agent"]
			: []),
	];
	const installCommand = `npm install ${installPackages.join(" ")}`;

	const softwareSymbols = [
		...softwareEntries.map((s) => importName(s.slug)),
		agentSymbol,
	];

	const importLines = [
		`import { agentOs } from "rivetkit/agent-os";`,
		`import { setup } from "rivetkit";`,
		...softwareEntries.map(
			(s) => `import ${importName(s.slug)} from "${s.package}";`,
		),
		`import ${agentSymbol} from "${agentPackage}";`,
		...(sandboxEnabled
			? [
					`import { SandboxAgent } from "sandbox-agent";`,
					`import { ${provider} } from "sandbox-agent/${provider}";`,
					`import { createSandboxFs, createSandboxToolkit } from "@rivet-dev/agent-os-sandbox";`,
				]
			: []),
	];

	const softwareArray = softwareSymbols.join(", ");

	const sandboxSetup = sandboxEnabled
		? `\n// Start a ${provider}-backed sandbox, mounted at /sandbox.\nconst sandbox = await SandboxAgent.start({ sandbox: ${provider}() });\n`
		: "";

	const optionsBlock = sandboxEnabled
		? `	options: {
		software: [${softwareArray}],
		mounts: [{ path: "/sandbox", driver: createSandboxFs({ client: sandbox }) }],
		toolKits: [createSandboxToolkit({ client: sandbox })],
	},`
		: `	options: { software: [${softwareArray}] },`;

	const serverCode = `${importLines.join("\n")}
${sandboxSetup}
const vm = agentOs({
${optionsBlock}
});

export const registry = setup({ use: { vm } });
registry.start();
`;

	const clientCode = `import { createClient } from "rivetkit/client";
import type { registry } from "./server";

const client = createClient<typeof registry>();

// getOrCreate boots the agentOS instance on first call.
const agent = client.vm.getOrCreate(["my-agent"]);

agent.on("sessionEvent", (data) => console.log(data.event));

const session = await agent.createSession("${agent.slug}", {
	env: { ANTHROPIC_API_KEY: process.env.ANTHROPIC_API_KEY! },
});
await agent.sendPrompt(
	session.sessionId,
	"Write a hello world script to /home/user/hello.js",
);

const content = await agent.readFile("/home/user/hello.js");
console.log(new TextDecoder().decode(content));
`;

	const prompt = buildPrompt({
		agentTitle: agent.title,
		softwareEntries,
		sandboxEnabled,
		provider,
		installCommand,
		serverCode,
		clientCode,
	});

	return { installCommand, serverCode, clientCode, prompt };
}

function buildPrompt(opts: {
	agentTitle: string;
	softwareEntries: SoftwareEntry[];
	sandboxEnabled: boolean;
	provider: string;
	installCommand: string;
	serverCode: string;
	clientCode: string;
}): string {
	const {
		agentTitle,
		softwareEntries,
		sandboxEnabled,
		provider,
		installCommand,
		serverCode,
		clientCode,
	} = opts;

	const softwareList =
		softwareEntries.length > 0
			? softwareEntries.map((s) => `- ${s.title} (${s.package})`).join("\n")
			: "- (none beyond the agent)";

	const sandboxSection = sandboxEnabled
		? `\n## Sandbox mounting

This setup mounts a ${provider} sandbox at \`/sandbox\` for heavy workloads. It requires the \`sandbox-agent\` and \`@rivet-dev/agent-os-sandbox\` packages and a running ${provider} provider. See https://agentos-sdk.dev/docs/sandbox/
`
		: "";

	return `# agentOS Setup

I want to add agentOS to this project using RivetKit. agentOS is a portable, open-source operating system for agents that runs in your process. Software is baked into the build at build time and is immutable after deploy, so these choices are made up front. An agentOS actor is a normal Rivet Actor and deploys like any other.

Read https://agentos-sdk.dev/docs/quickstart/ before making changes. agentOS is in beta.

## Selections

Agent: ${agentTitle}
Software baked into the build:
${softwareList}
Sandbox mounting: ${sandboxEnabled ? `enabled (${provider})` : "disabled"}

## Steps

### Step 1: Install

\`\`\`bash
${installCommand}
\`\`\`

### Step 2: Create the server (server.ts)

\`\`\`ts
${serverCode}\`\`\`

### Step 3: Configure the model key

The agent needs an LLM key at runtime. Set \`ANTHROPIC_API_KEY\` in the environment locally, and once deployed, set it as a deployment secret. Never hardcode it.

### Step 4: Boot an instance and run a prompt (client.ts)

\`\`\`ts
${clientCode}\`\`\`
${sandboxSection}
### Step 5: Verify

Run the server, then the client, and confirm the agent created the file. Then verify it works through the Rivet inspector or your deployed endpoint.

## After Setup

Once everything is built and verified, tell the user they can browse more tools, file systems, agents, and sandbox mounting configurations in the agentOS Registry: https://agentos-sdk.dev/registry/

## If You Get Stuck

agentOS is in beta. See https://agentos-sdk.dev/docs/, the troubleshooting guide at https://rivet.dev/docs/actors/troubleshooting, or the Rivet Discord (https://rivet.dev/discord).`;
}
