import * as Sentry from "@sentry/react";
import posthog, { type PostHog } from "posthog-js";
import { PostHogProvider } from "posthog-js/react";
import type { PropsWithChildren } from "react";
import { getConfig, useConfig } from "@/components";
import { cloudEnv, commonEnv } from "@/lib/env";

export function initThirdPartyProviders(router: unknown, debug: boolean) {
	const config = getConfig();

	let ph: PostHog | null = null;

	// init posthog
	if (config.posthog) {
		ph =
			posthog.init(config.posthog.apiKey, {
				api_host: config.posthog.apiHost,
				debug: debug,
			}) || null;
	}

	// init sentry
	if (config.sentry) {
		const integrations = [
			Sentry.tanstackRouterBrowserTracingIntegration(router),
			Sentry.browserTracingIntegration(),
		];
		if (ph) {
			integrations.push(
				ph.sentryIntegration({
					organization: "rivet-gg",
					projectId: commonEnv().VITE_APP_SENTRY_PROJECT_ID,
				}),
			);
		}

		Sentry.init({
			dsn: commonEnv().VITE_APP_SENTRY_DSN,
			tracesSampleRate: 1.0,
			integrations,
			tunnel: getConfig().sentry?.tunnel,
			tracePropagationTargets: [
				"api.rivet.dev",
				"cloud-api.rivet.dev",
				"api.staging.rivet.dev",
				"cloud-api.staging.rivet.dev",
				/localhost/,
			],
		});
	}
}

export function ThirdPartyProviders({ children }: PropsWithChildren) {
	const config = useConfig();

	const phProvider = config.posthog ? (
		<PostHogProvider client={posthog}>{children}</PostHogProvider>
	) : null;

	return phProvider;
}
