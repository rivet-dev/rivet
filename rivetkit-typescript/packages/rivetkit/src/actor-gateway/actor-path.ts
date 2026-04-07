/**
 * Actor gateway path parsing.
 *
 * Parses `/gateway/{...}` paths into either a direct actor ID path or a query
 * path with rvt-* query parameters. This is the TypeScript equivalent of the engine
 * parser at `engine/packages/guard/src/routing/actor_path.rs`.
 */
import * as cbor from "cbor-x";
import * as errors from "@/actor/errors";
import {
	type ActorGatewayQuery,
	type CrashPolicy,
	GetForKeyRequestSchema,
	GetOrCreateRequestSchema,
} from "@/client/query";

/**
 * The `rvt-` query parameter prefix is reserved for Rivet gateway routing.
 * All query parameters with this prefix are stripped before forwarding
 * requests to the actor, so actors will never see them.
 */
const RVT_PREFIX = "rvt-";

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
 * Format: `/gateway/{name}/{...path}?rvt-namespace=...&rvt-method=get|getOrCreate&rvt-key=...`
 *
 * The actor name is a clean path segment, and all routing params are rvt-*
 * query parameters that get stripped before forwarding to the actor. This path
 * must be resolved to a concrete actor ID before proxying, using the engine
 * control client's getWithKey or getOrCreateWithKey methods.
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

/**
 * Parse actor routing information from a gateway path.
 *
 * Returns a `ParsedDirectActorPath` or `ParsedQueryActorPath` depending on the
 * URL structure, or `null` if the path does not start with `/gateway/`.
 *
 * Detection heuristic: if any query parameter starts with `rvt-`, it is a query
 * path. Otherwise it is a direct actor ID path. The two cases are handled by
 * `parseQueryActorPath` and `parseDirectActorPath` respectively.
 *
 * This must stay in sync with the engine parser at
 * `engine/packages/guard/src/routing/actor_path.rs`.
 */
export function parseActorPath(path: string): ParsedActorPath | null {
	// Extract base path and raw query from the original path string directly,
	// without running through a URL parser, to preserve actor query params
	// byte-for-byte (no re-encoding of %20, +, etc.).
	const [basePath, rawQuery] = splitPathAndQuery(path);

	if (basePath.includes("//")) {
		return null;
	}

	const segments = basePath.split("/").filter((s) => s.length > 0);
	if (segments[0] !== "gateway") {
		return null;
	}

	const rawQueryStr = rawQuery ?? "";

	// Check if any raw query param key starts with rvt-.
	const hasRvt =
		rawQueryStr.length > 0 &&
		rawQueryStr.split("&").some((part) => {
			const key = part.split("=")[0];
			return key.startsWith(RVT_PREFIX);
		});

	if (hasRvt) {
		const rvtParams = extractRvtParamsFromRaw(rawQueryStr);
		const actorQueryString = stripRvtQueryParams(rawQueryStr);
		return parseQueryActorPath(
			basePath,
			segments,
			rvtParams,
			actorQueryString,
		);
	}

	// Direct path: pass the raw query string through unchanged.
	const rawQueryString = rawQuery !== null ? `?${rawQuery}` : "";
	return parseDirectActorPath(basePath, rawQueryString);
}

function parseDirectActorPath(
	basePath: string,
	rawQueryString: string,
): ParsedDirectActorPath | null {
	const segments = basePath.split("/").filter((s) => s.length > 0);

	if (segments.length < 2 || segments[0] !== "gateway") {
		return null;
	}

	const actorSegment = segments[1];
	if (actorSegment.length === 0) {
		return null;
	}

	let actorId: string;
	let token: string | undefined;

	const atPos = actorSegment.indexOf("@");
	if (atPos !== -1) {
		const rawActorId = actorSegment.slice(0, atPos);
		const rawToken = actorSegment.slice(atPos + 1);

		if (rawActorId.length === 0 || rawToken.length === 0) {
			return null;
		}

		try {
			actorId = decodeURIComponent(rawActorId);
			token = decodeURIComponent(rawToken);
		} catch {
			return null;
		}
	} else {
		try {
			actorId = decodeURIComponent(actorSegment);
		} catch {
			return null;
		}
		token = undefined;
	}

	const remainingPath = buildRemainingPath(basePath, rawQueryString, 2);

	return {
		type: "direct",
		actorId,
		token,
		remainingPath,
	};
}

function parseQueryActorPath(
	basePath: string,
	segments: string[],
	rvtParams: Array<[string, string]>,
	actorQueryString: string,
): ParsedQueryActorPath {
	const nameSegment = segments[1];
	if (!nameSegment || nameSegment.length === 0) {
		throw new errors.InvalidRequest(
			"query gateway actor name must not be empty",
		);
	}

	if (nameSegment.includes("@")) {
		throw new errors.InvalidRequest(
			"query gateway paths must not use @token syntax",
		);
	}

	let name: string;
	try {
		name = decodeURIComponent(nameSegment);
	} catch {
		throw new errors.InvalidRequest(
			"invalid percent-encoding for query gateway param 'name'",
		);
	}

	if (name.length === 0) {
		throw new errors.InvalidRequest(
			"query gateway actor name must not be empty",
		);
	}

	const rvt = extractRvtParams(rvtParams);
	const remainingPath = buildRemainingPath(basePath, actorQueryString, 2);

	return {
		type: "query",
		query: buildActorQuery(name, rvt),
		namespace: rvt.namespace,
		runnerName: rvt.runner,
		crashPolicy: rvt.crashPolicy,
		token: rvt.token,
		remainingPath,
	};
}

interface RvtParams {
	namespace: string;
	method: string;
	runner?: string;
	key: string[];
	input?: unknown;
	region?: string;
	crashPolicy?: CrashPolicy;
	token?: string;
}

function splitKey(raw: string | undefined): string[] {
	if (raw === undefined || raw === "") return [];
	return raw.split(",");
}

function extractRvtParams(rvtRaw: Array<[string, string]>): RvtParams {
	const params = new Map<string, string>();

	for (const [rawKey, value] of rvtRaw) {
		const stripped = rawKey.slice(RVT_PREFIX.length);

		if (
			stripped === "namespace" ||
			stripped === "method" ||
			stripped === "runner" ||
			stripped === "key" ||
			stripped === "input" ||
			stripped === "region" ||
			stripped === "crash-policy" ||
			stripped === "token"
		) {
			if (params.has(stripped)) {
				throw new errors.InvalidRequest(
					`duplicate query gateway param: ${rawKey}`,
				);
			}
			params.set(stripped, value);
		} else {
			throw new errors.InvalidRequest(
				`unknown query gateway param: ${rawKey}`,
			);
		}
	}

	const requireParam = (name: string): string => {
		const value = params.get(name);
		if (value === undefined) {
			throw new errors.InvalidRequest(
				`missing required param: rvt-${name}`,
			);
		}
		return value;
	};

	const namespace = requireParam("namespace");
	const method = requireParam("method");
	const runner = params.get("runner");
	const key = splitKey(params.get("key"));
	const region = params.get("region");
	const token = params.get("token");

	// Decode input CBOR if present.
	const inputRaw = params.get("input");
	let input: unknown;
	if (inputRaw !== undefined) {
		const inputBuffer = decodeBase64Url(inputRaw);
		try {
			input = cbor.decode(inputBuffer);
		} catch (cause) {
			throw new errors.InvalidRequest(
				`invalid query gateway input cbor: ${cause}`,
			);
		}
	}

	// Parse crash policy.
	const crashPolicyRaw = params.get("crash-policy");
	let crashPolicy: CrashPolicy | undefined;
	if (crashPolicyRaw !== undefined) {
		if (
			crashPolicyRaw !== "restart" &&
			crashPolicyRaw !== "sleep" &&
			crashPolicyRaw !== "destroy"
		) {
			throw new errors.InvalidRequest(
				`unknown crash policy: ${crashPolicyRaw}, expected restart, sleep, or destroy`,
			);
		}
		crashPolicy = crashPolicyRaw;
	}

	return {
		namespace,
		method,
		runner,
		key,
		input,
		region,
		crashPolicy,
		token,
	};
}

function buildActorQuery(name: string, rvt: RvtParams): ActorGatewayQuery {
	switch (rvt.method) {
		case "get": {
			if (
				rvt.input !== undefined ||
				rvt.region !== undefined ||
				rvt.crashPolicy !== undefined ||
				rvt.runner !== undefined
			) {
				throw new errors.InvalidRequest(
					"query gateway method=get does not allow rvt-input, rvt-region, rvt-crash-policy, or rvt-runner params",
				);
			}

			return {
				getForKey: GetForKeyRequestSchema.parse({
					name,
					key: rvt.key,
				}),
			};
		}
		case "getOrCreate": {
			if (rvt.runner === undefined) {
				throw new errors.InvalidRequest(
					"query gateway method=getOrCreate requires rvt-runner param",
				);
			}

			return {
				getOrCreateForKey: GetOrCreateRequestSchema.parse({
					name,
					key: rvt.key,
					input: rvt.input,
					region: rvt.region,
				}),
			};
		}
		default:
			throw new errors.InvalidRequest(
				`unknown method: ${rvt.method}, expected get or getOrCreate`,
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

/** Split a path into the base path and the raw query string (without `?`). Fragments are stripped. */
function splitPathAndQuery(path: string): [string, string | null] {
	const fragmentPos = path.indexOf("#");
	const pathNoFragment =
		fragmentPos !== -1 ? path.slice(0, fragmentPos) : path;
	const queryPos = pathNoFragment.indexOf("?");
	if (queryPos !== -1) {
		return [
			pathNoFragment.slice(0, queryPos),
			pathNoFragment.slice(queryPos + 1),
		];
	}
	return [pathNoFragment, null];
}

/**
 * Extract rvt-* params from a raw query string, decoding their values
 * using form-urlencoded rules (`+` as space, then percent-decode).
 */
function extractRvtParamsFromRaw(rawQuery: string): Array<[string, string]> {
	const params: Array<[string, string]> = [];
	for (const part of rawQuery.split("&")) {
		const eqPos = part.indexOf("=");
		const rawKey = eqPos !== -1 ? part.slice(0, eqPos) : part;
		const rawValue = eqPos !== -1 ? part.slice(eqPos + 1) : "";

		if (rawKey.startsWith(RVT_PREFIX)) {
			let decodedValue: string;
			try {
				decodedValue = decodeFormValue(rawValue);
			} catch {
				throw new errors.InvalidRequest(
					`invalid percent-encoding for query gateway param '${rawKey}'`,
				);
			}
			params.push([rawKey, decodedValue]);
		}
	}
	return params;
}

/** Decode a form-urlencoded value: treat `+` as space, then percent-decode. */
function decodeFormValue(raw: string): string {
	return decodeURIComponent(raw.replace(/\+/g, " "));
}

/**
 * Strip rvt-* params from a raw query string, preserving actor params
 * byte-for-byte without re-encoding.
 */
function stripRvtQueryParams(rawQuery: string): string {
	const actorParts = rawQuery.split("&").filter((part) => {
		if (part.length === 0) return false;
		const key = part.split("=")[0];
		return !key.startsWith(RVT_PREFIX);
	});
	return actorParts.length === 0 ? "" : `?${actorParts.join("&")}`;
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
