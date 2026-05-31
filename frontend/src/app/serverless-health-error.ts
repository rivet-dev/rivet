import z from "zod";

export const HEALTH_CHECK_FALLBACK_ERROR =
	"Health check failed. Verify the endpoint is reachable.";

const metadataSchema = z
	.object({
		kind: z.string().optional(),
		status_code: z.number().optional(),
		body: z.string().optional(),
		parse_error: z.string().optional(),
		runtime: z.string().optional(),
		version: z.union([z.string(), z.number()]).optional(),
		envoy_protocol_version: z.number().optional(),
		max_supported_envoy_protocol_version: z.number().optional(),
	})
	.partial();

const metadataErrorSchema = z.object({
	message: z.string().optional(),
	details: z.string().optional(),
	metadata: metadataSchema.optional(),
});

/**
 * Formats a serverless health check error envelope into a display string.
 *
 * The engine surfaces every failure variant as a stable
 * `{message, details, metadata}` envelope where `metadata.kind` discriminates
 * the variant (see `ServerlessMetadataErrorEnvelope` in
 * `engine/packages/pegboard/src/ops/serverless_metadata/fetch.rs`). The server
 * message is already human-readable for every variant, so we start with it and
 * append the extra context the message itself omits (response body, JSON parse
 * error). Falls back to a generic message only when nothing parseable is present.
 */
export function formatServerlessMetadataError(error: unknown): string {
	const parsed = metadataErrorSchema.safeParse(error);
	if (!parsed.success) {
		return HEALTH_CHECK_FALLBACK_ERROR;
	}

	const { message, details, metadata } = parsed.data;
	const kind = metadata?.kind;

	let result = message ?? "";

	if (kind === "non_success_status") {
		const body = metadata?.body?.trim();
		if (body) result = result ? `${result}: ${body}` : body;
	} else if (kind === "invalid_response_json") {
		const extra = metadata?.parse_error?.trim() || metadata?.body?.trim();
		if (extra) result = result ? `${result}: ${extra}` : extra;
	}

	if (details) result = result ? `${result} (${details})` : details;

	if (!result && kind) result = kind.replace(/_/g, " ");

	return result || HEALTH_CHECK_FALLBACK_ERROR;
}
