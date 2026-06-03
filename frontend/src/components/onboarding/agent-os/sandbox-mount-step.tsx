import { useController } from "react-hook-form";
import { DEFAULT_SANDBOX_PROVIDER, SANDBOX_PROVIDERS } from "./catalog";
import { SelectCard } from "./select-card";

export interface SandboxValue {
	enabled: boolean;
	provider?: string;
}

// Presentational sandbox-mounting selector. Off by default; when on, pick a
// provider mounted at /sandbox.
export function SandboxMount({
	value,
	onChange,
}: {
	value: SandboxValue;
	onChange: (value: SandboxValue) => void;
}) {
	const provider = value.provider ?? DEFAULT_SANDBOX_PROVIDER;

	return (
		<div className="flex flex-col gap-4">
			<SelectCard
				multi
				title="Mount a sandbox"
				description="Mount a full sandbox at /sandbox for heavy workloads like browsers or native compilation. agentOS itself stays lightweight; this is optional."
				selected={value.enabled}
				onSelect={() =>
					onChange({ ...value, enabled: !value.enabled })
				}
			/>
			{value.enabled ? (
				<div>
					<p className="text-xs text-muted-foreground mb-2">
						Provider (mounted at <code>/sandbox</code>)
					</p>
					<div className="grid grid-cols-2 gap-2">
						{SANDBOX_PROVIDERS.map((p) => (
							<SelectCard
								key={p.slug}
								title={p.title}
								description={p.description}
								selected={provider === p.slug}
								onSelect={() =>
									onChange({ ...value, provider: p.slug })
								}
							/>
						))}
					</div>
				</div>
			) : null}
		</div>
	);
}

// Wizard step: binds SandboxMount to the react-hook-form `sandbox` field.
export function SandboxMountStep() {
	const { field } = useController({ name: "sandbox" });
	const value = (field.value as SandboxValue) ?? {
		enabled: false,
		provider: DEFAULT_SANDBOX_PROVIDER,
	};
	return <SandboxMount value={value} onChange={field.onChange} />;
}
