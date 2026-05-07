import type { Story } from "@ladle/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RivetError } from "@rivetkit/engine-api-full";
import "../../../.ladle/ladle.css";
import { TooltipProvider } from "@/components";
import {
	buildInspectorTokenErrorMessage,
	OutdatedInspector,
	useInspectorGuard,
} from "./guard-connectable-inspector";

function OutdatedInspectorPreview({ error }: { error: unknown }) {
	return (
		<OutdatedInspector error={error}>
			<RenderGuard />
		</OutdatedInspector>
	);
}

function RenderGuard() {
	const node = useInspectorGuard();
	return <>{node}</>;
}

const queryClient = new QueryClient({
	defaultOptions: { queries: { retry: false, staleTime: Infinity } },
});

function Frame({ children }: { children: React.ReactNode }) {
	return (
		<QueryClientProvider client={queryClient}>
			<TooltipProvider>
				<div className="bg-background min-h-screen p-12">
					<div className="max-w-3xl space-y-8">{children}</div>
				</div>
			</TooltipProvider>
		</QueryClientProvider>
	);
}

function Section({
	title,
	children,
}: {
	title: string;
	children: React.ReactNode;
}) {
	return (
		<div className="space-y-2">
			<h3 className="text-sm font-medium text-muted-foreground">
				{title}
			</h3>
			<div className="rounded-md border bg-card p-4">{children}</div>
		</div>
	);
}

const LOCAL_METADATA = { type: "local", version: "2.0.40" };
const DEPLOYED_METADATA = { type: "deployed", version: "2.0.40" };
const OUTDATED_METADATA = { type: "deployed", version: "2.0.10" };

function rivetError(statusCode: number, body: unknown) {
	return new RivetError({
		message: typeof body === "object" && body !== null && "message" in body
			? String((body as { message: unknown }).message)
			: "Request failed",
		statusCode,
		body,
	});
}

// Real engine error codes. Sources:
//   engine/packages/pegboard/src/errors.rs
//   engine/packages/api-builder/src/errors.rs
//   engine/packages/api-public/src/errors.rs
//   engine/packages/namespace/src/errors.rs
//   engine/packages/guard-core/src/errors.rs

const KV_KEY_NOT_FOUND = rivetError(404, {
	group: "actor",
	code: "kv_key_not_found",
	message: "The KV key does not exist for this actor.",
});

const FORBIDDEN = rivetError(403, {
	group: "api",
	code: "forbidden",
	message: "Access denied",
});

const UNAUTHORIZED = rivetError(401, {
	group: "api",
	code: "unauthorized",
	message: "Authentication required",
});

const ACTOR_NOT_FOUND = rivetError(404, {
	group: "actor",
	code: "not_found",
	message: "The actor does not exist or was destroyed.",
});

const NAMESPACE_NOT_FOUND = rivetError(404, {
	group: "namespace",
	code: "not_found",
	message: "The namespace does not exist.",
});

const NO_RUNNER_CONFIG_CONFIGURED = rivetError(400, {
	group: "actor",
	code: "no_runner_config_configured",
	message:
		"No runner config with name 'serverless' are available in any datacenter for the namespace 'default'. Validate a provider is listed that matches the requested pool name.",
});

const SERVICE_UNAVAILABLE = rivetError(503, {
	group: "guard",
	code: "service_unavailable",
	message: "Service unavailable.",
});

const TUNNEL_RESPONSE_CLOSED = rivetError(502, {
	group: "guard",
	code: "tunnel_response_closed",
	message: "Actor tunnel closed before sending a response.",
});

const RATE_LIMIT = rivetError(429, {
	group: "guard",
	code: "rate_limit",
	message:
		"Too many requests to 'GET /actors/01HXK3TRAD8N5K9V2J6F1Z2N4Q/kv/keys/aW5zcGVjdG9yX3Rva2Vu' from IP 203.0.113.42.",
});

const API_INTERNAL_ERROR = rivetError(500, {
	group: "api",
	code: "internal_error",
	message: "An internal server error occurred",
});

const NETWORK_ERROR = new TypeError("Failed to fetch");

export const RivetKitOutdated: Story = () => (
	<Frame>
		<Section title="RivetKit version below minimum (takes precedence over any error)">
			{buildInspectorTokenErrorMessage({
				statusCode: 500,
				metadata: OUTDATED_METADATA,
				error: API_INTERNAL_ERROR,
			})}
		</Section>
	</Frame>
);

export const RivetKitOutdatedKvKeyNotFound: Story = () => (
	<Frame>
		<Section title="RivetKit outdated · underlying error is kv_key_not_found">
			{buildInspectorTokenErrorMessage({
				statusCode: 404,
				metadata: OUTDATED_METADATA,
				error: KV_KEY_NOT_FOUND,
			})}
		</Section>
	</Frame>
);

export const KvKeyNotFound: Story = () => (
	<Frame>
		<Section title="Inspector token KV key missing · RivetKit version is current (inspector likely disabled)">
			{buildInspectorTokenErrorMessage({
				statusCode: 404,
				metadata: DEPLOYED_METADATA,
				error: KV_KEY_NOT_FOUND,
			})}
		</Section>
	</Frame>
);

export const KvEndpointMissing: Story = () => (
	<Frame>
		<Section title="404 with no structured body · old engine without KV endpoint">
			{buildInspectorTokenErrorMessage({
				statusCode: 404,
				metadata: DEPLOYED_METADATA,
				error: new RivetError({
					statusCode: 404,
					body: "Not Found",
				}),
			})}
		</Section>
	</Frame>
);

export const Forbidden: Story = () => (
	<Frame>
		<Section title="403 · Inspector auth rejected (deployed)">
			{buildInspectorTokenErrorMessage({
				statusCode: 403,
				metadata: DEPLOYED_METADATA,
				error: FORBIDDEN,
			})}
		</Section>
	</Frame>
);

export const ForbiddenLocal: Story = () => (
	<Frame>
		<Section title="403 · local environment falls through to verbose error">
			{buildInspectorTokenErrorMessage({
				statusCode: 403,
				metadata: LOCAL_METADATA,
				error: FORBIDDEN,
			})}
		</Section>
	</Frame>
);

export const ActorNotFound: Story = () => (
	<Frame>
		<Section title="404 · actor.not_found (verbose engine error)">
			{buildInspectorTokenErrorMessage({
				statusCode: 404,
				metadata: DEPLOYED_METADATA,
				error: ACTOR_NOT_FOUND,
			})}
		</Section>
	</Frame>
);

export const NamespaceNotFound: Story = () => (
	<Frame>
		<Section title="404 · namespace.not_found">
			{buildInspectorTokenErrorMessage({
				statusCode: 404,
				metadata: DEPLOYED_METADATA,
				error: NAMESPACE_NOT_FOUND,
			})}
		</Section>
	</Frame>
);

export const Unauthorized: Story = () => (
	<Frame>
		<Section title="401 · api.unauthorized">
			{buildInspectorTokenErrorMessage({
				statusCode: 401,
				metadata: DEPLOYED_METADATA,
				error: UNAUTHORIZED,
			})}
		</Section>
	</Frame>
);

export const NoRunnerConfigConfigured: Story = () => (
	<Frame>
		<Section title="400 · actor.no_runner_config_configured">
			{buildInspectorTokenErrorMessage({
				statusCode: 400,
				metadata: DEPLOYED_METADATA,
				error: NO_RUNNER_CONFIG_CONFIGURED,
			})}
		</Section>
	</Frame>
);

export const ServiceUnavailable: Story = () => (
	<Frame>
		<Section title="503 · guard.service_unavailable">
			{buildInspectorTokenErrorMessage({
				statusCode: 503,
				metadata: DEPLOYED_METADATA,
				error: SERVICE_UNAVAILABLE,
			})}
		</Section>
	</Frame>
);

export const TunnelResponseClosed: Story = () => (
	<Frame>
		<Section title="502 · guard.tunnel_response_closed">
			{buildInspectorTokenErrorMessage({
				statusCode: 502,
				metadata: DEPLOYED_METADATA,
				error: TUNNEL_RESPONSE_CLOSED,
			})}
		</Section>
	</Frame>
);

export const RateLimit: Story = () => (
	<Frame>
		<Section title="429 · guard.rate_limit">
			{buildInspectorTokenErrorMessage({
				statusCode: 429,
				metadata: DEPLOYED_METADATA,
				error: RATE_LIMIT,
			})}
		</Section>
	</Frame>
);

export const InternalError: Story = () => (
	<Frame>
		<Section title="500 · api.internal_error">
			{buildInspectorTokenErrorMessage({
				statusCode: 500,
				metadata: DEPLOYED_METADATA,
				error: API_INTERNAL_ERROR,
			})}
		</Section>
	</Frame>
);

export const NonStructuredError: Story = () => (
	<Frame>
		<Section title="Unstructured error (e.g. network failure)">
			{buildInspectorTokenErrorMessage({
				metadata: DEPLOYED_METADATA,
				error: NETWORK_ERROR,
			})}
		</Section>
	</Frame>
);

export const OutdatedInspectorRivetKitTooOld: Story = () => (
	<Frame>
		<Section title="Token fetched OK but inspector protocol unavailable · RivetKit version below minimum">
			<OutdatedInspectorPreview error={undefined} />
		</Section>
	</Frame>
);

export const OutdatedInspectorWithEngineError: Story = () => (
	<Frame>
		<Section title="Post-token metadata fetch failed · verbose engine error">
			<OutdatedInspectorPreview error={TUNNEL_RESPONSE_CLOSED} />
		</Section>
	</Frame>
);

export const OutdatedInspectorNoStructuredError: Story = () => (
	<Frame>
		<Section title="Post-token metadata fetch failed · no structured error">
			<OutdatedInspectorPreview error={NETWORK_ERROR} />
		</Section>
	</Frame>
);

export const Gallery: Story = () => (
	<Frame>
		<Section title="RivetKit outdated (takes precedence)">
			{buildInspectorTokenErrorMessage({
				statusCode: 500,
				metadata: OUTDATED_METADATA,
				error: API_INTERNAL_ERROR,
			})}
		</Section>
		<Section title="RivetKit outdated · with actor.kv_key_not_found">
			{buildInspectorTokenErrorMessage({
				statusCode: 404,
				metadata: OUTDATED_METADATA,
				error: KV_KEY_NOT_FOUND,
			})}
		</Section>
		<Section title="actor.kv_key_not_found · current RivetKit">
			{buildInspectorTokenErrorMessage({
				statusCode: 404,
				metadata: DEPLOYED_METADATA,
				error: KV_KEY_NOT_FOUND,
			})}
		</Section>
		<Section title="KV endpoint missing · old engine (404, no engine body)">
			{buildInspectorTokenErrorMessage({
				statusCode: 404,
				metadata: DEPLOYED_METADATA,
				error: new RivetError({
					statusCode: 404,
					body: "Not Found",
				}),
			})}
		</Section>
		<Section title="api.forbidden">
			{buildInspectorTokenErrorMessage({
				statusCode: 403,
				metadata: DEPLOYED_METADATA,
				error: FORBIDDEN,
			})}
		</Section>
		<Section title="api.unauthorized">
			{buildInspectorTokenErrorMessage({
				statusCode: 401,
				metadata: DEPLOYED_METADATA,
				error: UNAUTHORIZED,
			})}
		</Section>
		<Section title="actor.not_found">
			{buildInspectorTokenErrorMessage({
				statusCode: 404,
				metadata: DEPLOYED_METADATA,
				error: ACTOR_NOT_FOUND,
			})}
		</Section>
		<Section title="namespace.not_found">
			{buildInspectorTokenErrorMessage({
				statusCode: 404,
				metadata: DEPLOYED_METADATA,
				error: NAMESPACE_NOT_FOUND,
			})}
		</Section>
		<Section title="actor.no_runner_config_configured">
			{buildInspectorTokenErrorMessage({
				statusCode: 400,
				metadata: DEPLOYED_METADATA,
				error: NO_RUNNER_CONFIG_CONFIGURED,
			})}
		</Section>
		<Section title="guard.service_unavailable">
			{buildInspectorTokenErrorMessage({
				statusCode: 503,
				metadata: DEPLOYED_METADATA,
				error: SERVICE_UNAVAILABLE,
			})}
		</Section>
		<Section title="guard.tunnel_response_closed">
			{buildInspectorTokenErrorMessage({
				statusCode: 502,
				metadata: DEPLOYED_METADATA,
				error: TUNNEL_RESPONSE_CLOSED,
			})}
		</Section>
		<Section title="guard.rate_limit">
			{buildInspectorTokenErrorMessage({
				statusCode: 429,
				metadata: DEPLOYED_METADATA,
				error: RATE_LIMIT,
			})}
		</Section>
		<Section title="api.internal_error">
			{buildInspectorTokenErrorMessage({
				statusCode: 500,
				metadata: DEPLOYED_METADATA,
				error: API_INTERNAL_ERROR,
			})}
		</Section>
		<Section title="Unstructured error (e.g. network failure)">
			{buildInspectorTokenErrorMessage({
				metadata: DEPLOYED_METADATA,
				error: NETWORK_ERROR,
			})}
		</Section>
		<Section title="Post-token · RivetKit version too old (no error, just unavailable)">
			<OutdatedInspectorPreview error={undefined} />
		</Section>
		<Section title="Post-token metadata fetch failed · verbose engine error">
			<OutdatedInspectorPreview error={TUNNEL_RESPONSE_CLOSED} />
		</Section>
		<Section title="Post-token metadata fetch failed · no structured error">
			<OutdatedInspectorPreview error={NETWORK_ERROR} />
		</Section>
	</Frame>
);
