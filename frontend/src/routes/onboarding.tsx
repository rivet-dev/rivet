import { createFileRoute, notFound, Outlet } from "@tanstack/react-router";
import { waitForClerk } from "@/lib/waitForClerk";

export const Route = createFileRoute("/onboarding")({
	component: RouteComponent,
	beforeLoad: async (route) => {
		if (__APP_TYPE__ !== "cloud") {
			throw notFound();
		}

		// Onboarding routes require authentication - wait for user and session
		await waitForClerk(route.context.clerk, { requireAuth: true });
	},
});

function RouteComponent() {
	return __APP_TYPE__ === "cloud" ? <Outlet /> : null;
}
