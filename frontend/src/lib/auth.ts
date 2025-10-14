import { Clerk } from "@clerk/clerk-js";
import { redirect } from "@tanstack/react-router";
import { cloudEnv } from "./env";

export const clerk =
	__APP_TYPE__ === "cloud"
		? new Clerk(cloudEnv().VITE_APP_CLERK_PUBLISHABLE_KEY)
		: (null as unknown as Clerk);

export const redirectToOrganization = async (clerk: Clerk) => {
	if (clerk.user) {
		if (clerk.organization) {
			throw redirect({
				to: "/orgs/$organization",
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
				params: { organization: orgs[0].organization.id },
			});
		}
		throw redirect({
			to: "/onboarding/choose-organization",
		});
	}

	return false;
};
