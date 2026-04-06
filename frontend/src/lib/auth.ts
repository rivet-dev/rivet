import type { Clerk } from "@clerk/clerk-js";
import { redirect } from "@tanstack/react-router";
import { cloudEnv } from "./env";

async function createClerk(): Promise<Clerk> {
	const { Clerk } = await import("@clerk/clerk-js");
	return new Clerk(cloudEnv().VITE_APP_CLERK_PUBLISHABLE_KEY);
}

export const clerkPromise: Promise<Clerk> =
	__APP_TYPE__ === "cloud"
		? createClerk()
		: Promise.resolve(null as unknown as Clerk);

// Resolved synchronously after clerkPromise settles (awaited in main.tsx before
// the router renders). Safe to use in any lazy-loaded chunk.
export let clerk: Clerk = null as unknown as Clerk;
clerkPromise.then(
	(instance) => { clerk = instance; },
	() => {},
);

export const redirectToOrganization = async (
	clerk: Clerk,
	search: Record<string, string>,
) => {
	if (clerk.user) {
		if (clerk.organization) {
			throw redirect({
				to: "/orgs/$organization",
				search: true,
				params: {
					organization: clerk.organization.id,
				},
			});
		}
		const { data: orgs } = await clerk.user.getOrganizationMemberships();

		if (orgs.length > 0) {
			await clerk.setActive({ organization: orgs[0].organization.id });
			throw redirect({
				to: "/orgs/$organization",
				search: true,
				params: { organization: orgs[0].organization.id },
			});
		}
		throw redirect({
			to: "/onboarding/choose-organization",
			search: true,
		});
	}

	return false;
};
