import type { CallToolResult } from "@modelcontextprotocol/sdk/types.js";

export interface InspectorFailure {
	title: string;
	message: string;
	group?: string;
	code?: string;
	hint: string;
	recoverable: boolean;
}

interface StructuredRivetError {
	group?: unknown;
	code?: unknown;
	message?: unknown;
}

export class InspectorToolError extends Error {
	readonly group?: string;
	readonly code?: string;

	constructor(message: string, group?: string, code?: string) {
		super(message);
		this.name = "InspectorToolError";
		this.group = group;
		this.code = code;
	}
}

function firstText(result: CallToolResult): string | undefined {
	for (const block of result.content ?? []) {
		if (block.type === "text" && block.text.trim())
			return block.text.trim();
	}
	return undefined;
}

// Tool handlers answer failures with `structuredContent.error` shaped like a
// RivetError, and only fall back to the text block when the host strips
// structured content.
export function toolResultError(
	result: CallToolResult,
	fallback: string,
): InspectorToolError {
	const structured = (result.structuredContent as { error?: unknown })?.error;
	const error = (structured ?? {}) as StructuredRivetError;
	const group = typeof error.group === "string" ? error.group : undefined;
	const code = typeof error.code === "string" ? error.code : undefined;
	const message =
		typeof error.message === "string" && error.message
			? error.message
			: (firstText(result) ?? fallback);
	return new InspectorToolError(message, group, code);
}

const HINTS: Record<string, string> = {
	"inspector_session.principal_saturated":
		"Too many Inspector sessions are open for this connection. Close another embedded Inspector, or wait for its session to expire, then try again.",
	"inspector_session.invalid_or_expired":
		"The temporary session behind this panel expired. Retrying mints a fresh one.",
	"inspector_session.stale_token":
		"This panel is holding a rotated session token. Retrying mints a fresh one.",
	"inspector_session.renewal_limit":
		"This session hit its maximum lifetime. Retrying starts a new one from scratch.",
	"target.target_ambiguous":
		"The connection did not resolve to a single namespace. Ask for the Inspector again while naming the organization, project, and namespace.",
	"actor.not_found":
		"The actor is gone or was never created. Ask for the actor list to pick a live one.",
};

const GROUP_HINTS: Record<string, string> = {
	inspector_session:
		"The temporary Inspector session could not be established. Retrying mints a fresh one.",
	acl: "The connected Rivet token is missing the Inspector scope. Reconnect the Rivet MCP server with an inspector-capable token.",
	target: "The connection did not resolve to a single Rivet namespace.",
	actor: "The actor could not be resolved on this namespace.",
};

const UNRECOVERABLE = new Set(["inspector_session.principal_saturated", "acl"]);

export function describeFailure(
	error: unknown,
	title: string,
): InspectorFailure {
	const tool = error instanceof InspectorToolError ? error : undefined;
	const group = tool?.group;
	const code = tool?.code;
	const qualified = group && code ? `${group}.${code}` : undefined;
	const message =
		error instanceof Error && error.message
			? error.message
			: "The Inspector could not be reached.";
	const hint =
		(qualified ? HINTS[qualified] : undefined) ??
		(group ? GROUP_HINTS[group] : undefined) ??
		"This is usually transient. Retrying mints a fresh session; if it keeps failing, reconnect the Rivet MCP server.";
	const recoverable = !(
		(qualified && UNRECOVERABLE.has(qualified)) ||
		(group && UNRECOVERABLE.has(group))
	);
	return { title, message, group, code, hint, recoverable };
}

export function failureSummary(failure: InspectorFailure): string {
	const qualified =
		failure.group && failure.code
			? `${failure.group}.${failure.code}`
			: undefined;
	return [
		`Rivet Actor Inspector: ${failure.title}`,
		qualified
			? `Error: ${qualified} - ${failure.message}`
			: `Error: ${failure.message}`,
		failure.hint,
	].join("\n");
}
