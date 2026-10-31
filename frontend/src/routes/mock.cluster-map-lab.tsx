import { createFileRoute, notFound } from "@tanstack/react-router";
import { ClusterMapLab } from "@/components/cluster-onboarding/cluster-map-lab";

// Dev-only harness for the cluster map animation. See ClusterMapLab.

export const Route = createFileRoute("/mock/cluster-map-lab")({
	component: RouteComponent,
	beforeLoad: () => {
		if (!import.meta.env.DEV && !import.meta.env.VITE_DEMO_ROUTE) {
			throw notFound();
		}
	},
});

function RouteComponent() {
	return (
		<div className="min-h-screen bg-background text-foreground">
			<ClusterMapLab />
		</div>
	);
}
