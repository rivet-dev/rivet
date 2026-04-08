import * as Sentry from "@sentry/react";
import type { PropsWithChildren } from "react";
import { Suspense, lazy } from "react";
import { getConfig, useConfig } from "@/components";
import { commonEnv } from "@/lib/env";

export function initThirdPartyProviders(router: unknown, debug: boolean) {
	const config = getConfig();

	// PostHog is cloud-only; guarded by __APP_TYPE__ so it's DCE'd in the engine build.
	if (__APP_TYPE__ === "cloud" && config.posthog) {
		import("posthog-js").then(({ default: posthog }) => {
			posthog.init(config.posthog!.apiKey, {
				api_host: config.posthog!.apiHost,
				debug: debug,
			});

			// init sentry with posthog integration
			if (config.sentry) {
				initSentry(router, posthog);
			}
		});
	} else if (config.sentry) {
		// init sentry without posthog
		initSentry(router, null);
	}
}

function initSentry(router: unknown, ph: { sentryIntegration: (opts: object) => unknown } | null) {
	const config = getConfig();
	if (!config.sentry) return;

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
		tunnel: config.sentry?.tunnel || undefined,
		tracePropagationTargets: [
			"api.rivet.dev",
			"cloud-api.rivet.dev",
			"api.staging.rivet.dev",
			"cloud-api.staging.rivet.dev",
			/localhost/,
		],
	});
}

const PostHogWrapper = lazy(() => {
	// Guard with __APP_TYPE__ so Rollup DCEs the posthog imports in the engine build.
	if (__APP_TYPE__ !== "cloud") {
		return Promise.resolve({
			default: function Noop({ children }: PropsWithChildren) {
				return <>{children}</>;
			},
		});
	}
	return Promise.all([import("posthog-js"), import("posthog-js/react")]).then(
		([{ default: posthog }, { PostHogProvider }]) => ({
			default: function PostHogWrapperInner({ children }: PropsWithChildren) {
				return <PostHogProvider client={posthog}>{children}</PostHogProvider>;
			},
		}),
	);
});

export function ThirdPartyProviders({ children }: PropsWithChildren) {
	const config = useConfig();

	if (!config.posthog) {
		return <>{children}</>;
	}

	return (
		<Suspense fallback={<>{children}</>}>
			<PostHogWrapper>{children}</PostHogWrapper>
		</Suspense>
	);
}
