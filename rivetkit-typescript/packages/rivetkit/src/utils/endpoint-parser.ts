import { z } from "zod";

export interface ParsedEndpoint {
	endpoint: string;
	namespace: string | undefined;
	token: string | undefined;
}

/**
 * Parses an endpoint URL that may contain auth syntax for namespace and token.
 *
 * Supports formats like:
 * - `https://namespace:token@api.rivet.dev`
 * - `https://namespace@api.rivet.dev` (namespace only, no token)
 * - `https://api.rivet.dev` (no auth)
 * - `https://namespace:token@api.rivet.dev/path` (with path)
 *
 * Query strings and fragments are not allowed as they may conflict with
 * runtime parameters.
 */
export function zodParseEndpoint(endpoint: string): ParsedEndpoint {
	// Parse the URL
	const url = new URL(endpoint);

	// Reject query strings
	if (url.search) {
		throw new z.ZodError([
			{
				code: "custom",
				message: "endpoint cannot contain a query string",
				path: ["endpoint"],
			},
		]);
	}

	// Reject fragments
	if (url.hash) {
		throw new z.ZodError([
			{
				code: "custom",
				message: "endpoint cannot contain a fragment",
				path: ["endpoint"],
			},
		]);
	}

	// Extract namespace and token from username and password
	// URL stores these as percent-encoded, so we need to decode them
	const namespace = url.username ? decodeURIComponent(url.username) : undefined;
	const token = url.password ? decodeURIComponent(url.password) : undefined;

	// Reject token without namespace (e.g., https://:token@api.rivet.dev)
	if (token && !namespace) {
		throw new z.ZodError([
			{
				code: "custom",
				message: "endpoint cannot have a token without a namespace",
				path: ["endpoint"],
			},
		]);
	}

	// Strip auth from the URL by clearing username and password
	url.username = "";
	url.password = "";

	// Get the cleaned endpoint without auth
	const cleanedEndpoint = url.toString();

	return {
		endpoint: cleanedEndpoint,
		namespace,
		token,
	};
}

/**
 * Zod schema that parses an endpoint URL string and extracts namespace/token from HTTP auth syntax.
 *
 * Input: `"https://namespace:token@api.rivet.dev/path"`
 * Output: `{ endpoint: "https://api.rivet.dev/path", namespace: "namespace", token: "token" }`
 */
export const EndpointSchema = z.string().transform((endpoint): ParsedEndpoint => {
	return zodParseEndpoint(endpoint);
});

export type EndpointSchemaInput = z.input<typeof EndpointSchema>;
export type EndpointSchemaOutput = z.output<typeof EndpointSchema>;

/**
 * Zod refinement that validates namespace/token aren't specified both in the endpoint URL
 * and as separate config options.
 */
export function zodCheckDuplicateCredentials(
	resolvedEndpoint: ParsedEndpoint,
	config: { namespace?: string; token?: string },
	ctx: z.RefinementCtx,
): void {
	// Check if endpoint contains namespace but namespace is also specified in config
	if (resolvedEndpoint.namespace && config.namespace) {
		ctx.addIssue({
			code: "custom",
			message:
				"cannot specify namespace both in endpoint URL and as a separate config option",
			path: ["namespace"],
		});
	}

	// Check if endpoint contains token but token is also specified in config
	if (resolvedEndpoint.token && config.token) {
		ctx.addIssue({
			code: "custom",
			message:
				"cannot specify token both in endpoint URL and as a separate config option",
			path: ["token"],
		});
	}
}

