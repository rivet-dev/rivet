import type { Story } from "@ladle/react";
import { useState } from "react";
import "../../../../.ladle/ladle.css";
import { AgentSelect } from "./agent-select-step";
import {
	type AgentOsSelections,
	buildAgentOsSetup,
} from "./build-agent-os-setup";
import { DEFAULT_AGENT, DEFAULT_PACKAGES } from "./catalog";
import { SandboxMount, type SandboxValue } from "./sandbox-mount-step";
import { SoftwareSelect } from "./software-select-step";

// Integration story: the three agentOS selection steps wired to the pure
// `buildAgentOsSetup` generator, so the generated install command / server.ts /
// client.ts / handoff prompt update live as selections change. This is the unit
// that matters (selection -> generated code); the individual card lists are
// trivial on their own.
function Harness({ initial }: { initial: AgentOsSelections }) {
	const [agent, setAgent] = useState(initial.agent);
	const [packages, setPackages] = useState<string[]>(initial.packages);
	const [sandbox, setSandbox] = useState<SandboxValue>(initial.sandbox);

	const setup = buildAgentOsSetup({ agent, packages, sandbox });

	return (
		<div className="bg-background min-h-screen p-10 text-foreground">
			<div className="grid grid-cols-2 gap-8 max-w-6xl">
				<div className="flex flex-col gap-6">
					<Section title="Agent">
						<AgentSelect value={agent} onChange={setAgent} />
					</Section>
					<Section title="Software">
						<SoftwareSelect value={packages} onChange={setPackages} />
					</Section>
					<Section title="Sandbox & mounts">
						<SandboxMount value={sandbox} onChange={setSandbox} />
					</Section>
				</div>
				<div className="flex flex-col gap-6">
					<Section title="Install command">
						<Code>{setup.installCommand}</Code>
					</Section>
					<Section title="server.ts">
						<Code>{setup.serverCode}</Code>
					</Section>
					<Section title="client.ts">
						<Code>{setup.clientCode}</Code>
					</Section>
					<Section title="Handoff prompt">
						<Code>{setup.prompt}</Code>
					</Section>
				</div>
			</div>
		</div>
	);
}

function Section({
	title,
	children,
}: {
	title: string;
	children: React.ReactNode;
}) {
	return (
		<div className="flex flex-col gap-2">
			<h2 className="text-sm font-semibold text-foreground">{title}</h2>
			{children}
		</div>
	);
}

function Code({ children }: { children: string }) {
	return (
		<pre className="overflow-auto rounded-md border border-border bg-muted/30 p-3 text-xs text-foreground whitespace-pre-wrap">
			{children}
		</pre>
	);
}

export const Playground: Story = () => (
	<Harness
		initial={{
			agent: DEFAULT_AGENT,
			packages: DEFAULT_PACKAGES,
			sandbox: { enabled: false, provider: "docker" },
		}}
	/>
);

export const WithSandboxAndExtras: Story = () => (
	<Harness
		initial={{
			agent: DEFAULT_AGENT,
			packages: ["common", "ripgrep", "jq", "git"],
			sandbox: { enabled: true, provider: "docker" },
		}}
	/>
);
