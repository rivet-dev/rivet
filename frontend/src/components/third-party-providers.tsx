import * as Sentry from "@sentry/react";
import posthog, { type PostHog } from "posthog-js";
import { PostHogProvider } from "posthog-js/react";
import type { PropsWithChildren } from "react";
import { getConfig, useConfig } from "@/components";

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

export function ThirdPartyProviders({ children }: PropsWithChildren) {
	const config = useConfig();

	const phProvider = config.posthog ? (
		<PostHogProvider client={posthog}>{children}</PostHogProvider>
	) : null;

	return phProvider;
}
