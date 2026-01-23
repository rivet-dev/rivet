import { createFileRoute, redirect } from "@tanstack/react-router";
import { Content } from "@/app/layout";
import { RouteLayout } from "@/app/route-layout";

export const Route = createFileRoute("/onboarding/choose-organization")({
	component: RouteComponent,
	beforeLoad: async ({ context }) => {
		const RELOAD_KEY = "clerk-session-reload-count";

		// After SSO, there's a race condition where Clerk redirects here before
		// the session is fully established. If we detect this (user exists from
		// API perspective but no local session), reload to let Clerk sync state.
		if (!context.clerk.session) {
			const MAX_RELOADS = 3;
			const reloadCount = Number(
				sessionStorage.getItem(RELOAD_KEY) || "0",
			);

			if (reloadCount < MAX_RELOADS) {
				console.log(
					`[choose-organization] No session yet, reloading page to sync Clerk state (attempt ${reloadCount + 1}/${MAX_RELOADS})`,
				);
				sessionStorage.setItem(RELOAD_KEY, String(reloadCount + 1));
				window.location.reload();
				// Return a never-resolving promise to prevent further execution
				return new Promise(() => {});
			}

			// Max reloads reached, clear counter and show error
			sessionStorage.removeItem(RELOAD_KEY);
			console.error(
				"[choose-organization] No session after max reload attempts",
			);
			throw new Error(
				"Unable to establish session. Please try signing in again.",
			);
		}

		// Clear reload counter on success
		sessionStorage.removeItem(RELOAD_KEY);

		if (context.clerk.organization) {
			throw redirect({
				to: "/orgs/$organization",
				params: { organization: context.clerk.organization.id },
				search: true,
			});
		}

		const org = await context.clerk.createOrganization({
			name: `${context.clerk.user?.firstName || context.clerk.user?.primaryEmailAddress?.emailAddress.split("@")[0] || "Anonymous"}'s Organization`,
		});

		await context.clerk.setActive({ organization: org.id });
		await context.clerk.session?.reload();

		throw redirect({
			to: "/orgs/$organization",
			params: { organization: org.id },
			search: true,
		});
	},
});

function RouteComponent() {
	return (
		<RouteLayout>
			<Content className="flex flex-col items-center justify-safe-center">
				<div className="w-full sm:w-96">
					Creating your organization...
				</div>
			</Content>
		</RouteLayout>
	);
}
