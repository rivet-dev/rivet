import { createFileRoute, notFound } from "@tanstack/react-router";
import { SidebarLab } from "@/components/cluster-onboarding/sidebar-lab";

// Dev-only comparison of the sidebar namespace-switcher variants. See SidebarLab.

export const Route = createFileRoute("/mock/sidebar-lab")({
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
			<SidebarLab />
		</div>
	);
}
