import { useState } from "react";
import { ClusterSidebar, type ClusterView } from "./cluster-sidebar";
import {
	NAMESPACE_VARIANTS,
	type NamespaceId,
	type NamespaceVariant,
} from "./namespace-picker";

/**
 * Side-by-side comparison of the sidebar namespace-switcher variants. Each
 * column is a live sidebar: open the dropdown, switch namespaces, click actors.
 */
export function SidebarLab() {
	return (
		<div className="mx-auto flex max-w-6xl flex-col gap-6 px-6 py-8">
			<div>
				<h1 className="text-xl font-semibold">
					Sidebar namespace switcher
				</h1>
				<p className="mt-1 text-sm text-muted-foreground">
					Cluster pages · divider · namespace · actors. Four
					placements for the namespace dropdown.
				</p>
			</div>
			<div className="grid grid-cols-2 gap-6 lg:grid-cols-4">
				{NAMESPACE_VARIANTS.map((v) => (
					<LabColumn
						key={v.id}
						variant={v.id}
						label={v.label}
						blurb={v.blurb}
					/>
				))}
			</div>
		</div>
	);
}

function LabColumn({
	variant,
	label,
	blurb,
}: {
	variant: NamespaceVariant;
	label: string;
	blurb: string;
}) {
	const [view, setView] = useState<ClusterView>("actor:agentos");
	const [namespace, setNamespace] = useState<NamespaceId>("production");
	return (
		<div className="flex flex-col gap-3">
			<div>
				<p className="text-sm font-semibold">{label}</p>
				<p className="text-xs text-muted-foreground">{blurb}</p>
			</div>
			<div className="overflow-hidden rounded-lg border bg-background">
				<ClusterSidebar
					view={view}
					onViewChange={setView}
					namespaceVariant={variant}
					namespace={namespace}
					onNamespaceChange={setNamespace}
				/>
				<div className="h-6" />
			</div>
		</div>
	);
}
