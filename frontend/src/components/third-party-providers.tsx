import * as Sentry from "@sentry/react";
import { type PropsWithChildren, Suspense, lazy } from "react";
import { getConfig, useConfig } from "@/components";
import { commonEnv } from "@/lib/env";
import { initPosthog } from "@/lib/posthog";

export async function initThirdPartyProviders(router: unknown, debug: boolean) {
	const config = getConfig();

	let ph = null;

	if (config.posthog) {
		ph = await initPosthog(config.posthog.apiKey, config.posthog.apiHost, debug);
	}

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
			environment: commonEnv().VITE_APP_SENTRY_ENV,
			tunnel: getConfig().sentry?.tunnel || undefined,
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

const LazyPostHogProvider = lazy(() =>
	Promise.all([import("posthog-js"), import("posthog-js/react")]).then(
		([{ default: posthog }, { PostHogProvider }]) => ({
			default: ({ children }: PropsWithChildren) => (
				<PostHogProvider client={posthog}>{children}</PostHogProvider>
			),
		}),
	),
);

export function ThirdPartyProviders({ children }: PropsWithChildren) {
	const config = useConfig();

	if (!config.posthog) {
		return children;
	}

	return (
		<Suspense fallback={children}>
			<LazyPostHogProvider>{children}</LazyPostHogProvider>
		</Suspense>
	);
}
