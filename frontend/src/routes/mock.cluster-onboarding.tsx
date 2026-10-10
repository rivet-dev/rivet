import { createFileRoute, notFound } from "@tanstack/react-router";
import z from "zod";
import { ClusterOnboardingFlow } from "@/components/cluster-onboarding/cluster-onboarding-flow";

// Dev-only design mock of the cluster onboarding flow. Lives outside the
// authed `_context` tree so it renders in every flavor without a session.
// Visit /mock/cluster-onboarding (create page) or ?view=dashboard.

export const Route = createFileRoute("/mock/cluster-onboarding")({
	component: RouteComponent,
	validateSearch: z.object({
		view: z.enum(["create", "dashboard"]).optional(),
	}),
	beforeLoad: () => {
		// Design mock: available on the dev server and in demo builds
		// (`VITE_DEMO_ROUTE` set at build time). Real production builds
		// leave the flag unset, so this route 404s there.
		if (!import.meta.env.DEV && !import.meta.env.VITE_DEMO_ROUTE) {
			throw notFound();
		}
	},
});

function RouteComponent() {
	const { view } = Route.useSearch();
	return (
		<div className="min-h-screen bg-background text-foreground">
			<ClusterOnboardingFlow initialView={view} />
		</div>
	);
}
