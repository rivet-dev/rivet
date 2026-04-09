import { createFileRoute, redirect } from "@tanstack/react-router";
import { VerifyEmailPending } from "@/app/verify-email-pending";
import { authClient } from "@/lib/auth";
import { features } from "@/lib/features";

export const Route = createFileRoute("/verify-email-pending")({
	component: VerifyEmailPending,
	beforeLoad: async () => {
		if (!features.auth) return;

		const session = await authClient.getSession();

		if (!session.data) {
			throw redirect({ to: "/login" });
		}

		// Already verified — send them to the app.
		if (session.data.user.emailVerified) {
			throw redirect({ to: "/" });
		}
	},
});
