/**
 * Actor gateway path parsing.
 *
 * Parses `/gateway/{...}` paths into either a direct actor ID path or a query
 * path with matrix parameters. This is the TypeScript equivalent of the engine
 * parser at `engine/packages/guard/src/routing/actor_path.rs`.
 */
import * as cbor from "cbor-x";
import { z } from "zod/v4";
import * as errors from "@/actor/errors";
import {
	type ActorGatewayQuery,
	type CrashPolicy,
	CrashPolicySchema,
	GetForKeyRequestSchema,
	GetOrCreateRequestSchema,
} from "./protocol/query";

/**
 * A direct actor path targets a specific actor by its ID.
 * Format: `/gateway/{actorId}[@{token}]/{...path}`
 *
 * The actor ID is extracted directly from the URL and no query resolution is
 * needed. This is the path format used by resolved handles and connections
 * that already know their target actor.
 */
export interface ParsedDirectActorPath {
	type: "direct";
	actorId: string;
	token?: string;
	remainingPath: string;
}

/**
 * A query actor path resolves to an actor via a key-based lookup.
 * Format: `/gateway/{name};namespace={ns};method={get|getOrCreate};key={k}[;input=...][;region=...]/{...path}`
 *
 * The actor name is the path segment prefix (before the first `;`), and all
 * routing params are matrix params on the same segment. This path must be
 * resolved to a concrete actor ID before proxying, using the manager driver's
 * getWithKey or getOrCreateWithKey methods.
 *
 * This is the engine-side reference implementation's TypeScript equivalent.
 * See `engine/packages/guard/src/routing/actor_path.rs` for the Rust counterpart.
 */
export interface ParsedQueryActorPath {
	type: "query";
	query: ActorGatewayQuery;
	namespace: string;
	runnerName?: string;
	crashPolicy?: CrashPolicy;
	token?: string;
	remainingPath: string;
}

export type ParsedActorPath = ParsedDirectActorPath | ParsedQueryActorPath;

const GatewayQueryMethodSchema = z.enum(["get", "getOrCreate"]);

const GatewayQueryPathSchema = z
	.object({
		name: z.string(),
		namespace: z.string(),
		method: GatewayQueryMethodSchema,
		runnerName: z.string().optional(),
		key: z.array(z.string()).optional(),
		input: z.unknown().optional(),
		region: z.string().optional(),
		crashPolicy: CrashPolicySchema.optional(),
		token: z.string().optional(),
	})
	.strict();

/**
 * Parse actor routing information from a gateway path.
 *
 * Returns a `ParsedDirectActorPath` or `ParsedQueryActorPath` depending on the
 * URL structure, or `null` if the path does not start with `/gateway/`.
 *
 * Detection heuristic: if the second path segment (after "gateway") contains a
 * semicolon, it is a query path with matrix params. Otherwise it is a direct
 * actor ID path. The two cases are handled by `parseQueryActorPath` and
 * `parseDirectActorPath` respectively.
 *
 * This must stay in sync with the engine parser at
 * `engine/packages/guard/src/routing/actor_path.rs`.
 */
export function parseActorPath(path: string): ParsedActorPath | null {
	// Find query string position (everything from ? onwards, but before fragment)
	const queryPos = path.indexOf("?");
	const fragmentPos = path.indexOf("#");

	// Extract query string (excluding fragment)
	let queryString = "";
	if (queryPos !== -1) {
		if (fragmentPos !== -1 && queryPos < fragmentPos) {
			queryString = path.slice(queryPos, fragmentPos);
		} else {
			queryString = path.slice(queryPos);
		}
	}

	// Extract base path (before query and fragment)
	let basePath = path;
	if (queryPos !== -1) {
		basePath = path.slice(0, queryPos);
	} else if (fragmentPos !== -1) {
		basePath = path.slice(0, fragmentPos);
	}

	// Check for double slashes (invalid path)
	if (basePath.includes("//")) {
		return null;
	}

	const segments = basePath.split("/");
	if (segments[1] !== "gateway") {
		return null;
	}

	// Check the second segment (after "gateway") to distinguish query paths from
	// direct paths. Query paths have matrix params: /gateway/{name};namespace=...;method=...
	const nameSegment = segments[2];
	if (nameSegment && nameSegment.includes(";")) {
		return parseQueryActorPath(nameSegment, basePath, queryString);
	}

	return parseDirectActorPath(basePath, queryString);
}

function parseDirectActorPath(
	basePath: string,
	queryString: string,
): ParsedDirectActorPath | null {
	// Split the path into segments
	const segments = basePath.split("/").filter((s) => s.length > 0);

	// Check minimum required segments: gateway, {actor_id}
	if (segments.length < 2) {
		return null;
	}

	// Verify the first segment is "gateway"
	if (segments[0] !== "gateway") {
		return null;
	}

	// Extract actor_id segment (may contain @token)
	const actorSegment = segments[1];

	// Check for empty actor segment
	if (actorSegment.length === 0) {
		return null;
	}

	// Parse actor_id and optional token from the segment
	let actorId: string;
	let token: string | undefined;

	const atPos = actorSegment.indexOf("@");
	if (atPos !== -1) {
		// Pattern: /gateway/{actor_id}@{token}/{...path}
		const rawActorId = actorSegment.slice(0, atPos);
		const rawToken = actorSegment.slice(atPos + 1);

		// Check for empty actor_id or token
		if (rawActorId.length === 0 || rawToken.length === 0) {
			return null;
		}

		// URL-decode both actor_id and token
		try {
			actorId = decodeURIComponent(rawActorId);
			token = decodeURIComponent(rawToken);
		} catch (_e) {
			// Invalid URL encoding
			return null;
		}
	} else {
		// Pattern: /gateway/{actor_id}/{...path}
		// URL-decode actor_id
		try {
			actorId = decodeURIComponent(actorSegment);
		} catch (_e) {
			// Invalid URL encoding
			return null;
		}
		token = undefined;
	}

	// Calculate remaining path
	// The remaining path starts after /gateway/{actor_id[@token]}/
	let prefixLen = 0;
	for (let i = 0; i < 2; i++) {
		prefixLen += 1 + segments[i].length; // +1 for the slash
	}

	// Extract the remaining path preserving trailing slashes
	let remainingBase: string;
	if (prefixLen < basePath.length) {
		remainingBase = basePath.slice(prefixLen);
	} else {
		remainingBase = "/";
	}

	// Ensure remaining path starts with /
	let remainingPath: string;
	if (remainingBase.length === 0 || !remainingBase.startsWith("/")) {
		remainingPath = `/${remainingBase}${queryString}`;
	} else {
		remainingPath = `${remainingBase}${queryString}`;
	}

	return {
		type: "direct",
		actorId,
		token,
		remainingPath,
	};
}

function parseQueryActorPath(
	nameSegment: string,
	basePath: string,
	queryString: string,
): ParsedQueryActorPath {
	if (nameSegment.includes("@")) {
		throw new errors.InvalidRequest(
			"query gateway paths must not use @token syntax",
		);
	}

	const params = parseQueryGatewayParams(nameSegment);
	const remainingPath = buildRemainingPath(basePath, queryString, 2);

	return {
		type: "query",
		query: buildActorQueryFromGatewayParams(params),
		namespace: params.namespace,
		runnerName: params.runnerName,
		crashPolicy: params.crashPolicy,
		token: params.token,
		remainingPath,
	};
}

function parseQueryGatewayParams(
	nameSegment: string,
): z.infer<typeof GatewayQueryPathSchema> {
	const semicolonPos = nameSegment.indexOf(";");
	const rawName = nameSegment.slice(0, semicolonPos);
	const paramsStr = nameSegment.slice(semicolonPos + 1);

	const decodedName = decodeMatrixParamValue(rawName, "name");
	if (decodedName.length === 0) {
		throw new errors.InvalidRequest(
			"query gateway actor name must not be empty",
		);
	}

	const params: Record<string, unknown> = { name: decodedName };

	if (paramsStr.length > 0) {
		for (const rawParam of paramsStr.split(";")) {
			const equalsPos = rawParam.indexOf("=");
			if (equalsPos === -1) {
				throw new errors.InvalidRequest(
					`query gateway param is missing '=': ${rawParam}`,
				);
			}

			const name = rawParam.slice(0, equalsPos);
			const rawValue = rawParam.slice(equalsPos + 1);

			if (name === "name") {
				throw new errors.InvalidRequest(
					"duplicate query gateway param: name",
				);
			}

			if (!isQueryGatewayParamName(name)) {
				throw new errors.InvalidRequest(
					`unknown query gateway param: ${name}`,
				);
			}

			if (Object.hasOwn(params, name)) {
				throw new errors.InvalidRequest(
					`duplicate query gateway param: ${name}`,
				);
			}

			params[name] = parseQueryGatewayParamValue(name, rawValue);
		}
	}

	const parseResult = GatewayQueryPathSchema.safeParse(params);
	if (!parseResult.success) {
		throw new errors.InvalidRequest(
			parseResult.error.issues[0]?.message ??
				"invalid query gateway params",
		);
	}

	if (
		parseResult.data.method === "get" &&
		(Object.hasOwn(params, "input") ||
			Object.hasOwn(params, "region") ||
			Object.hasOwn(params, "crashPolicy") ||
			Object.hasOwn(params, "runnerName"))
	) {
		throw new errors.InvalidRequest(
			"query gateway method=get does not allow input, region, crashPolicy, or runnerName params",
		);
	}

	if (
		parseResult.data.method === "getOrCreate" &&
		!Object.hasOwn(params, "runnerName")
	) {
		throw new errors.InvalidRequest(
			"query gateway method=getOrCreate requires runnerName param",
		);
	}

	return parseResult.data;
}

function buildActorQueryFromGatewayParams(
	params: z.infer<typeof GatewayQueryPathSchema>,
): ActorGatewayQuery {
	const key = params.key ?? [];

	if (params.method === "get") {
		return {
			getForKey: GetForKeyRequestSchema.parse({
				name: params.name,
				key,
			}),
		};
	}

	return {
		getOrCreateForKey: GetOrCreateRequestSchema.parse({
			name: params.name,
			key,
			input: params.input,
			region: params.region,
		}),
	};
}

function isQueryGatewayParamName(
	name: string,
): name is
	| "namespace"
	| "method"
	| "runnerName"
	| "key"
	| "input"
	| "region"
	| "crashPolicy"
	| "token" {
	return (
		name === "namespace" ||
		name === "method" ||
		name === "runnerName" ||
		name === "key" ||
		name === "input" ||
		name === "region" ||
		name === "crashPolicy" ||
		name === "token"
	);
}

function parseQueryGatewayParamValue(
	name:
		| "namespace"
		| "method"
		| "runnerName"
		| "key"
		| "input"
		| "region"
		| "crashPolicy"
		| "token",
	rawValue: string,
): unknown {
	if (name === "key") {
		return rawValue
			.split(",")
			.map((component) => decodeMatrixParamValue(component, name));
	}

	if (name === "input") {
		const inputBuffer = decodeBase64Url(
			decodeMatrixParamValue(rawValue, name),
		);

		try {
			return cbor.decode(inputBuffer);
		} catch (cause) {
			throw new errors.InvalidRequest(
				`invalid query gateway input cbor: ${cause}`,
			);
		}
	}

	return decodeMatrixParamValue(rawValue, name);
}

function decodeMatrixParamValue(rawValue: string, name: string): string {
	try {
		return decodeURIComponent(rawValue);
	} catch {
		throw new errors.InvalidRequest(
			`invalid percent-encoding for query gateway param '${name}'`,
		);
	}
}

function decodeBase64Url(value: string): Uint8Array {
	if (!/^[A-Za-z0-9_-]*$/.test(value) || value.length % 4 === 1) {
		throw new errors.InvalidRequest(
			"invalid base64url in query gateway input",
		);
	}

	const paddingLength = (4 - (value.length % 4 || 4)) % 4;
	const base64 =
		value.replace(/-/g, "+").replace(/_/g, "/") + "=".repeat(paddingLength);

	if (typeof Buffer !== "undefined") {
		return new Uint8Array(Buffer.from(base64, "base64"));
	}

	const binary = atob(base64);
	const buffer = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i++) {
		buffer[i] = binary.charCodeAt(i);
	}

	return buffer;
}

function buildRemainingPath(
	basePath: string,
	queryString: string,
	consumedSegments: number,
): string {
	const segments = basePath
		.split("/")
		.filter((segment) => segment.length > 0);

	let prefixLen = 0;
	for (let i = 0; i < consumedSegments; i++) {
		prefixLen += 1 + segments[i].length;
	}

	let remainingBase: string;
	if (prefixLen < basePath.length) {
		remainingBase = basePath.slice(prefixLen);
	} else {
		remainingBase = "/";
	}

	if (remainingBase.length === 0 || !remainingBase.startsWith("/")) {
		return `/${remainingBase}${queryString}`;
	}

	return `${remainingBase}${queryString}`;
}
