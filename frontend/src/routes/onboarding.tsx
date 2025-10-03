import { createFileRoute, notFound, Outlet } from "@tanstack/react-router";
import { waitForClerk } from "@/lib/waitForClerk";

export const Route = createFileRoute("/onboarding")({
	component: RouteComponent,
	beforeLoad: async (route) => {
		if (__APP_TYPE__ !== "cloud") {
			throw notFound();
		}

		await waitForClerk(route.context.clerk);
	},
});

function RouteComponent() {
	return __APP_TYPE__ === "cloud" ? <Outlet /> : null;
}
