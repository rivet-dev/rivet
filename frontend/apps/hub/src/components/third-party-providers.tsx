import { useAuth } from "@/domains/auth/contexts/auth";
import { UTCDate } from "@date-fns/utc";
import * as Sentry from "@sentry/react";
import { usePostHog } from "posthog-js/react";
import { useEffect } from "react";

export function IdentifyUser() {
	const posthog = usePostHog();
	const { profile } = useAuth();

	useEffect(() => {
		const identity = profile?.identity;
		if (identity) {
			const user = {
				name: identity.displayName,
				email: identity.linkedAccounts.find((x) => x.email)?.email
					?.email,
				joinTs: new UTCDate(identity.joinTs).toISOString(),
				avatar: identity.avatarUrl,
				isAdmin: identity.isAdmin,
			};

			posthog.identify(`user:${identity.identityId}`, user);
			Sentry.setUser(user);
		}
	}, [posthog, profile]);

	return null;
}
