import type { Story } from "@ladle/react";
import type { Rivet } from "@rivetkit/engine-api-full";
import "../../.ladle/ladle.css";
import { HealthCheckFailure } from "./serverless-connection-check";

type Failure =
	Rivet.RunnerConfigsServerlessHealthCheckResponseFailure["failure"];

// Each fixture mirrors the `{message, details, metadata}` envelope the engine
// emits for a `ServerlessMetadataError` variant (see
// `engine/packages/pegboard/src/ops/serverless_metadata/fetch.rs`). Before the
// fix every one of these rendered as "Unknown error" because the formatter
// matched an obsolete discriminated-union shape.

const INVALID_REQUEST: Failure = {
	error: {
		message: "invalid serverless metadata request",
		metadata: { kind: "invalid_request" },
	},
};

const REQUEST_FAILED: Failure = {
	error: {
		message: "failed to reach serverless endpoint",
		metadata: { kind: "request_failed" },
	},
};

const REQUEST_TIMED_OUT: Failure = {
	error: {
		message: "serverless metadata request timed out",
		metadata: { kind: "request_timed_out" },
	},
};

const NON_SUCCESS_STATUS: Failure = {
	error: {
		message: "serverless metadata request returned status 502",
		metadata: {
			kind: "non_success_status",
			status_code: 502,
			body: "Bad Gateway: upstream connection refused",
		},
	},
};

const INVALID_RESPONSE_JSON: Failure = {
	error: {
		message: "serverless metadata response is not valid JSON",
		metadata: {
			kind: "invalid_response_json",
			body: "<html><body>504 Gateway Timeout</body></html>",
			parse_error: "expected value at line 1 column 1",
		},
	},
};

const INVALID_RESPONSE_SCHEMA: Failure = {
	error: {
		message: "serverless runtime express version 0.1.0 is unsupported",
		metadata: {
			kind: "invalid_response_schema",
			runtime: "express",
			version: "0.1.0",
		},
	},
};

const INVALID_ENVOY_PROTOCOL_VERSION: Failure = {
	error: {
		message: "envoy protocol version 5 is not supported (max supported: 4)",
		metadata: {
			kind: "invalid_envoy_protocol_version",
			envoy_protocol_version: 5,
			max_supported_envoy_protocol_version: 4,
		},
	},
};

// Unrecognized envelope: the server message still surfaces even when the
// `kind` is one the frontend has never seen.
const UNKNOWN_KIND: Failure = {
	error: {
		message: "something new went wrong",
		metadata: { kind: "some_future_variant", extra: "context" },
	},
};

function Frame({ children }: { children: React.ReactNode }) {
	return (
		<div className="bg-background min-h-screen p-12">
			<div className="max-w-3xl space-y-6 text-destructive text-sm">
				{children}
			</div>
		</div>
	);
}

function Section({ title, error }: { title: string; error: Failure }) {
	return (
		<div className="space-y-1">
			<h3 className="text-xs font-medium text-muted-foreground">
				{title}
			</h3>
			<div className="rounded-md border border-destructive p-4">
				<HealthCheckFailure error={error} />
			</div>
		</div>
	);
}

export const Gallery: Story = () => (
	<Frame>
		<Section title="invalid_request" error={INVALID_REQUEST} />
		<Section title="request_failed" error={REQUEST_FAILED} />
		<Section title="request_timed_out" error={REQUEST_TIMED_OUT} />
		<Section title="non_success_status" error={NON_SUCCESS_STATUS} />
		<Section title="invalid_response_json" error={INVALID_RESPONSE_JSON} />
		<Section
			title="invalid_response_schema"
			error={INVALID_RESPONSE_SCHEMA}
		/>
		<Section
			title="invalid_envoy_protocol_version"
			error={INVALID_ENVOY_PROTOCOL_VERSION}
		/>
		<Section
			title="unknown kind (forward compatible)"
			error={UNKNOWN_KIND}
		/>
	</Frame>
);

export const InvalidEnvoyProtocolVersion: Story = () => (
	<Frame>
		<Section
			title="invalid_envoy_protocol_version"
			error={INVALID_ENVOY_PROTOCOL_VERSION}
		/>
	</Frame>
);

export const NonSuccessStatus: Story = () => (
	<Frame>
		<Section title="non_success_status" error={NON_SUCCESS_STATUS} />
	</Frame>
);
