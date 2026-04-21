import { redirect } from "@tanstack/react-router";
import { organizationClient } from "better-auth/client/plugins";
import { createAuthClient } from "better-auth/react";
import { cloudEnv } from "./env";

const createClient = () =>
	createAuthClient({
		baseURL: cloudEnv().VITE_APP_CLOUD_API_URL,
		plugins: [organizationClient()],
	});

type AuthClient = ReturnType<typeof createClient>;

export const authClient: AuthClient =
	__APP_TYPE__ === "cloud" ? createClient() : (null as unknown as AuthClient);

export const redirectToOrganization = async (
	search: Record<string, string>,
) => {
	const session = await authClient.getSession();
	if (session.data) {
		const orgs = await authClient.organization.list();

		if (orgs.data && orgs.data.length > 0) {
			await authClient.organization.setActive({
				organizationId: orgs.data[0].id,
			});
			throw redirect({
				to: "/orgs/$organization",
				search: true,
				params: { organization: orgs.data[0].id },
			});
		}
		throw redirect({
			to: "/onboarding/choose-organization",
			search: true,
		});
	}

	return false;
};
