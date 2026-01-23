import { z } from "zod";

export interface ParsedEndpoint {
	endpoint: string;
	namespace: string | undefined;
	token: string | undefined;
}

export interface TryParseEndpointOptions {
	/** The endpoint URL string to parse */
	endpoint: string;
	/** Path prefix for error messages (default: ["endpoint"]) */
	path?: (string | number)[];
	/** Namespace from config, to check for duplicate specification */
	namespace?: string;
	/** Token from config, to check for duplicate specification */
	token?: string;
}

/**
 * Parses an endpoint URL that may contain auth syntax for namespace and token.
 *
 * Uses ctx.addIssue for clean error reporting in Zod transforms.
 *
 * Supports formats like:
 * - `https://namespace:token@api.rivet.dev`
 * - `https://namespace@api.rivet.dev` (namespace only, no token)
 * - `https://api.rivet.dev` (no auth)
 * - `https://namespace:token@api.rivet.dev/path` (with path)
 *
 * Query strings and fragments are not allowed as they may conflict with
 * runtime parameters.
 *
 * @param ctx - Zod refinement context for error reporting
 * @param options - Parsing options including endpoint, path, and config values
 * @returns ParsedEndpoint on success, undefined on error (after adding issues to ctx)
 */
export function tryParseEndpoint(
	ctx: z.RefinementCtx,
	options: TryParseEndpointOptions,
): ParsedEndpoint | undefined {
	const { endpoint, path = ["endpoint"], namespace: configNamespace, token: configToken } = options;
	// Parse the URL
	let url: URL;
	try {
		url = new URL(endpoint);
	} catch {
		ctx.addIssue({
			code: "custom",
			message: `invalid URL: ${endpoint}`,
			path,
		});
		return undefined;
	}

	// Reject query strings
	if (url.search) {
		ctx.addIssue({
			code: "custom",
			message: "endpoint cannot contain a query string",
			path,
		});
		return undefined;
	}

	// Reject fragments
	if (url.hash) {
		ctx.addIssue({
			code: "custom",
			message: "endpoint cannot contain a fragment",
			path,
		});
		return undefined;
	}

	// Extract namespace and token from username and password
	// URL stores these as percent-encoded, so we need to decode them
	const namespace = url.username
		? decodeURIComponent(url.username)
		: undefined;
	const token = url.password ? decodeURIComponent(url.password) : undefined;

	// Reject token without namespace (e.g., https://:token@api.rivet.dev)
	if (token && !namespace) {
		ctx.addIssue({
			code: "custom",
			message: "endpoint cannot have a token without a namespace",
			path,
		});
		return undefined;
	}

	// Check for duplicate credentials (specified both in URL and config)
	if (namespace && configNamespace) {
		ctx.addIssue({
			code: "custom",
			message:
				"cannot specify namespace both in endpoint URL and as a separate config option",
			path: ["namespace"],
		});
	}
	if (token && configToken) {
		ctx.addIssue({
			code: "custom",
			message:
				"cannot specify token both in endpoint URL and as a separate config option",
			path: ["token"],
		});
	}

	// Build cleaned endpoint without auth.
	// We construct it manually instead of using url.username = "" because
	// some edge runtimes (Convex, Cloudflare Workers) don't support setting
	// username/password on URL objects.
	let cleanedEndpoint = `${url.protocol}//${url.host}${url.pathname}`;
	// Remove trailing slash if present (unless it's the root path)
	if (cleanedEndpoint.endsWith("/") && url.pathname !== "/") {
		cleanedEndpoint = cleanedEndpoint.slice(0, -1);
	}

	return {
		endpoint: cleanedEndpoint,
		namespace,
		token,
	};
}

