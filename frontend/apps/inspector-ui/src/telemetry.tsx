import * as Sentry from "@sentry/react";
import { type ComponentType, Fragment, type PropsWithChildren } from "react";
import { initPosthog } from "@/lib/posthog";

declare const __MCP_VERSION__: string;

export interface McpAppTelemetry {
	sentry?: { dsn: string; environment?: string; tunnel: string };
	posthog?: { apiKey: string; apiHost: string };
}

export function readMcpAppTelemetry(): McpAppTelemetry | null {
	const element = document.getElementById("RIVET_MCP_CONFIG");
	if (!element?.textContent) return null;
	try {
		const parsed: unknown = JSON.parse(element.textContent);
		if (!parsed || typeof parsed !== "object") return null;
		return parsed as McpAppTelemetry;
	} catch {
		return null;
	}
}

// Resolves before the app mounts. `McpInspector` connects to the host exactly
// once, so it must not be mounted under a boundary that can remount it.
export async function initMcpTelemetry(
	telemetry: McpAppTelemetry | null,
): Promise<ComponentType<PropsWithChildren>> {
	if (telemetry?.sentry) {
		Sentry.init({
			dsn: telemetry.sentry.dsn,
			// The host's CSP allows only the server that served this bundle, so
			// events go through its relay instead of straight to Sentry.
			tunnel: telemetry.sentry.tunnel,
			environment: telemetry.sentry.environment,
			release: `rivet-mcp@${__MCP_VERSION__}`,
			// Hosts frame the app with `sandbox="allow-scripts"`, so there is no
			// same-origin document to trace against.
			tracesSampleRate: 0,
		});
		Sentry.setTag("surface", "mcp-inspector-app");
	}

	if (!telemetry?.posthog) return Fragment;
	try {
		const [{ default: posthog }, { PostHogProvider }] = await Promise.all([
			import("posthog-js"),
			import("posthog-js/react"),
		]);
		await initPosthog(
			telemetry.posthog.apiKey,
			telemetry.posthog.apiHost,
			false,
			// Remote config and session replay arrive as <script> tags from the
			// relay origin, which the host's CSP does not cover for scripts.
			{ disable_external_dependency_loading: true },
		);
		// This shares the dashboard's project, and the sandboxed iframe cannot
		// persist a distinct id, so every open is a new anonymous user. Tag the
		// surface so dashboard metrics can exclude them.
		posthog.register({
			surface: "mcp-inspector-app",
			mcp_version: __MCP_VERSION__,
		});
		return ({ children }: PropsWithChildren) => (
			<PostHogProvider client={posthog}>{children}</PostHogProvider>
		);
	} catch (error) {
		Sentry.captureException(error);
		return Fragment;
	}
}
