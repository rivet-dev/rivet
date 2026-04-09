import { createFileRoute, redirect } from "@tanstack/react-router";
import { z } from "zod";
import { VerifyEmailPending } from "@/app/verify-email-pending";
import { authClient } from "@/lib/auth";
import { features } from "@/lib/features";

export const Route = createFileRoute("/verify-email-pending")({
	component: VerifyEmailPending,
	validateSearch: z.object({ email: z.string().optional() }),
	beforeLoad: async ({ search }) => {
		if (!features.auth) return;

		const session = await authClient.getSession();

		// No session and no email from sign-up — nothing to verify.
		if (!session.data && !search.email) {
			throw redirect({ to: "/login" });
		}

		// Already verified — send them to the app.
		if (session.data?.user.emailVerified) {
			throw redirect({ to: "/" });
		}
	},
});
