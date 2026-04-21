import { createFileRoute, notFound, Outlet, redirect } from "@tanstack/react-router";
import { authClient } from "@/lib/auth";
import { features } from "@/lib/features";

export const Route = createFileRoute("/onboarding")({
	component: RouteComponent,
	beforeLoad: async () => {
		if (!features.auth) {
			throw notFound();
		}

		const session = await authClient.getSession();
		if (!session.data) {
			throw redirect({ to: "/login" });
		}
	},
});

function RouteComponent() {
	return features.auth ? <Outlet /> : null;
}
