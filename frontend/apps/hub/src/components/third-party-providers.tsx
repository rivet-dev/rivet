import { router } from "@/app";
import { useAuth } from "@/domains/auth/contexts/auth";
import { UTCDate } from "@date-fns/utc";
import { getConfig, useConfig } from "@rivet-gg/components";
import * as Sentry from "@sentry/react";
import posthog, { type PostHog } from "posthog-js";
import { PostHogProvider, usePostHog } from "posthog-js/react";
import { type PropsWithChildren, useEffect } from "react";

export function initThirdPartyProviders() {
	const config = getConfig();

	let ph: PostHog | null = null;

	// init posthog
	if (config.posthog) {
		ph =
			posthog.init(config.posthog.apiKey, {
				api_host: config.posthog.apiHost,
				debug: import.meta.env.DEV,
			}) || null;
	}

	// init sentry
	if (config.sentry) {
		const integrations = [
			Sentry.tanstackRouterBrowserTracingIntegration(router),
		];
		if (ph) {
			integrations.push(
				ph.sentryIntegration({
					organization: "rivet-gg",
					projectId: Number.parseInt(config.sentry.projectId, 10),
				}),
			);
		}

		Sentry.init({
			dsn: config.sentry.dsn,
			integrations,
		});
	}
}

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

export function ThirdPartyProviders({ children }: PropsWithChildren) {
	const config = useConfig();

	const phProvider = config.posthog ? (
		<PostHogProvider client={posthog}>{children}</PostHogProvider>
	) : null;

	return phProvider;
}
