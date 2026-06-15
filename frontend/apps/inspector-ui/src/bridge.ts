// PostMessage protocol between the dashboard shell and this iframe.
//
// The Zod schemas live in rivetkit and are re-exported below so that
// the inspector-ui SPA, custom tab bundles (via `rivetkit/inspector-tab`),
// and the dashboard's iframe message handler all share a single source
// of truth. Adding a new envelope variant requires editing rivetkit only;
// the change lands on every consumer in lockstep.
//
// `v` versions the envelope only. New envelope shapes (e.g. streaming
// responses) get a `v: 2` variant added to the unions in rivetkit; both
// peers accept any version they recognize.

import {
	ACTOR_ID_PARAM,
	type InspectorTabDescriptor,
	POSTMESSAGE_PROTOCOL_VERSION,
	SHELL_ORIGIN_PARAM,
	type ShellToTabMessage,
	ShellToTabMessageSchema,
	type TabToShellMessage,
	TabToShellMessageSchema,
	type V1Init,
	type V1SetActiveTab,
	type V1TabsAvailable,
} from "rivetkit/inspector-tab";

// Re-export under the names this app already uses so consumers don't move.
export type { InspectorTabDescriptor };
export type ShellToIframeMessage = ShellToTabMessage;
export type IframeToShellMessage = TabToShellMessage;
export type InitMessage = V1Init;
export type SetActiveTabMessage = V1SetActiveTab;
export type TabsAvailableMessage = V1TabsAvailable;

export const shellToIframeMessageSchema = ShellToTabMessageSchema;
export const iframeToShellMessageSchema = TabToShellMessageSchema;
export const PROTOCOL_VERSION = POSTMESSAGE_PROTOCOL_VERSION;

export { SHELL_ORIGIN_PARAM, ACTOR_ID_PARAM };

// Returns the trusted shell origin for this iframe, or null if no
// shellOrigin was provided in the URL (same-origin shell case).
export function getTrustedShellOrigin(): string | null {
	const param = new URLSearchParams(window.location.search).get(
		SHELL_ORIGIN_PARAM,
	);
	if (!param) return null;
	try {
		return new URL(param).origin;
	} catch {
		return null;
	}
}

export function getInitialActorId(): string | null {
	return new URLSearchParams(window.location.search).get(ACTOR_ID_PARAM);
}
