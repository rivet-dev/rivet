import { useController } from "react-hook-form";
import { AGENTS, DEFAULT_AGENT } from "./catalog";
import { SelectCard } from "./select-card";

// Presentational single-select for the coding agent. Pi is available; the rest
// render disabled with a "Coming soon" badge.
export function AgentSelect({
	value,
	onChange,
}: {
	value: string;
	onChange: (slug: string) => void;
}) {
	return (
		<div className="flex flex-col gap-2">
			{AGENTS.map((agent) => (
				<SelectCard
					key={agent.slug}
					title={agent.title}
					description={agent.description}
					badge={
						agent.status === "coming-soon" ? "Coming soon" : undefined
					}
					selected={value === agent.slug}
					disabled={agent.status !== "available"}
					onSelect={() => onChange(agent.slug)}
				/>
			))}
		</div>
	);
}

// Wizard step: binds AgentSelect to the react-hook-form `agent` field.
export function AgentSelectStep() {
	const { field } = useController({ name: "agent" });
	return (
		<AgentSelect
			value={(field.value as string) ?? DEFAULT_AGENT}
			onChange={field.onChange}
		/>
	);
}
