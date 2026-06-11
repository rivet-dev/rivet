// Data provider for the agentOS inspector.
//
// Two implementations behind one shape:
//   - `createFixtureAgentOsSource` (ACTIVE) — serves the `fixtures.ts` data.
//   - `createLiveAgentOsSource` (SEAM) — calls the inspector's `executeAction`
//     to reach real agentOS actor RPCs. Wired to live data later (with Andrew).
//
// NOTE: agentOS data rides the inspector WebSocket via `executeAction`, NOT
// HTTP/fetch, so MSW (`?mock=1`) cannot intercept it. This fixture source is the
// only viable mock seam for the prototype, and is also why each tab component
// takes its data as props (so Ladle stories render straight from fixtures).

import { queryOptions, useQuery } from "@tanstack/react-query";
import { createContext, type ReactNode, useContext } from "react";
import { features } from "@/lib/features";
import type { ActorId } from "../queries";
import {
	AGENT_OS_MANIFEST,
	CONNECTIONS,
	FILE_CONTENTS,
	FILESYSTEM,
	INVOCATIONS,
	METADATA,
	MOUNTS,
	PROCESSES,
	SESSIONS,
	SOFTWARE,
	TOOLKITS,
	TRANSCRIPTS,
} from "./fixtures";
import type {
	AgentOsManifest,
	AgentOsMetadata,
	ConnInfo,
	FileContent,
	FsNode,
	MountInfo,
	ProcessInfo,
	SessionSummary,
	SoftwareBundle,
	ToolsData,
	TranscriptEvent,
} from "./types";

export const agentOsQueryKeys = {
	manifest: (id: ActorId) => ["actor", id, "agent-os", "manifest"] as const,
	sessions: (id: ActorId) => ["actor", id, "agent-os", "sessions"] as const,
	transcript: (id: ActorId, sessionId: string) =>
		["actor", id, "agent-os", "transcript", sessionId] as const,
	filesystem: (id: ActorId) =>
		["actor", id, "agent-os", "filesystem"] as const,
	fileContent: (id: ActorId, path: string) =>
		["actor", id, "agent-os", "file", path] as const,
	processes: (id: ActorId) => ["actor", id, "agent-os", "processes"] as const,
	tools: (id: ActorId) => ["actor", id, "agent-os", "tools"] as const,
	software: (id: ActorId) => ["actor", id, "agent-os", "software"] as const,
	mounts: (id: ActorId) => ["actor", id, "agent-os", "mounts"] as const,
	connections: (id: ActorId) =>
		["actor", id, "agent-os", "connections"] as const,
	metadata: (id: ActorId) => ["actor", id, "agent-os", "metadata"] as const,
};

/**
 * Prototype detection: behind the feature flag, an actor is treated as agentOS
 * when flagged as a demo via `?agentos=1` or a `localStorage` allowlist
 * (`AGENT_OS_DEMO_ACTORS`, comma-separated actor ids). This avoids hijacking
 * every actor in the dashboard. The live source replaces this by probing the
 * real manifest RPC.
 */
function isAgentOsDemoActor(actorId: ActorId): boolean {
	if (!features.agentOsInspector) return false;
	if (typeof window === "undefined") return false;
	try {
		const params = new URLSearchParams(window.location.search);
		if (params.get("agentos") === "1") return true;
		const allow = window.localStorage.getItem("AGENT_OS_DEMO_ACTORS");
		if (allow) {
			return allow
				.split(",")
				.map((s) => s.trim())
				.filter(Boolean)
				.includes(String(actorId));
		}
	} catch {
		// Ignore SSR / storage-access errors; treat as not-agentOS.
	}
	return false;
}

/**
 * Prototype grid filter: which actor *names* count as agentOS instances. Gated
 * by the feature flag. Reads a comma-separated `AGENT_OS_DEMO_NAMES` allowlist
 * from localStorage; if unset, falls back to a name heuristic (`/agent/i`) so
 * the All/Actors/Agents filter works out of the box. The live impl reads `kind`
 * per actor from the engine.
 */
export function isAgentOsDemoName(name: string): boolean {
	if (!features.agentOsInspector) return false;
	if (typeof window === "undefined") return false;
	try {
		const allow = window.localStorage.getItem("AGENT_OS_DEMO_NAMES");
		if (allow != null && allow.trim() !== "") {
			return allow
				.split(",")
				.map((s) => s.trim())
				.filter(Boolean)
				.includes(name);
		}
		return /agent/i.test(name);
	} catch {
		return false;
	}
}

export function createFixtureAgentOsSource() {
	return {
		manifestQueryOptions: (actorId: ActorId) =>
			queryOptions({
				staleTime: Infinity,
				queryKey: agentOsQueryKeys.manifest(actorId),
				queryFn: (): AgentOsManifest | null =>
					isAgentOsDemoActor(actorId) ? AGENT_OS_MANIFEST : null,
			}),
		sessionsQueryOptions: (actorId: ActorId) =>
			queryOptions({
				staleTime: Infinity,
				queryKey: agentOsQueryKeys.sessions(actorId),
				queryFn: (): SessionSummary[] => SESSIONS,
			}),
		transcriptQueryOptions: (actorId: ActorId, sessionId: string) =>
			queryOptions({
				staleTime: Infinity,
				queryKey: agentOsQueryKeys.transcript(actorId, sessionId),
				queryFn: (): TranscriptEvent[] => TRANSCRIPTS[sessionId] ?? [],
			}),
		filesystemQueryOptions: (actorId: ActorId) =>
			queryOptions({
				staleTime: Infinity,
				queryKey: agentOsQueryKeys.filesystem(actorId),
				queryFn: (): FsNode => FILESYSTEM,
			}),
		fileContentQueryOptions: (actorId: ActorId, path: string) =>
			queryOptions({
				staleTime: Infinity,
				queryKey: agentOsQueryKeys.fileContent(actorId, path),
				queryFn: (): FileContent | null => FILE_CONTENTS[path] ?? null,
			}),
		processesQueryOptions: (actorId: ActorId) =>
			queryOptions({
				staleTime: Infinity,
				queryKey: agentOsQueryKeys.processes(actorId),
				queryFn: (): ProcessInfo[] => PROCESSES,
			}),
		toolsQueryOptions: (actorId: ActorId) =>
			queryOptions({
				staleTime: Infinity,
				queryKey: agentOsQueryKeys.tools(actorId),
				queryFn: (): ToolsData => ({
					toolkits: TOOLKITS,
					invocations: INVOCATIONS,
				}),
			}),
		softwareQueryOptions: (actorId: ActorId) =>
			queryOptions({
				staleTime: Infinity,
				queryKey: agentOsQueryKeys.software(actorId),
				queryFn: (): SoftwareBundle[] => SOFTWARE,
			}),
		mountsQueryOptions: (actorId: ActorId) =>
			queryOptions({
				staleTime: Infinity,
				queryKey: agentOsQueryKeys.mounts(actorId),
				queryFn: (): MountInfo[] => MOUNTS,
			}),
		connectionsQueryOptions: (actorId: ActorId) =>
			queryOptions({
				staleTime: Infinity,
				queryKey: agentOsQueryKeys.connections(actorId),
				queryFn: (): ConnInfo[] => CONNECTIONS,
			}),
		metadataQueryOptions: (actorId: ActorId) =>
			queryOptions({
				staleTime: Infinity,
				queryKey: agentOsQueryKeys.metadata(actorId),
				queryFn: (): AgentOsMetadata => METADATA,
			}),
	};
}

export type AgentOsInspectorSource = ReturnType<
	typeof createFixtureAgentOsSource
>;

/**
 * Integration seam for live data. Wire each query to the matching agentOS actor
 * RPC through the inspector's `executeAction`. Action names and the
 * ACP→TranscriptEvent transform are finalized when this is wired up.
 */
export function createLiveAgentOsSource(
	executeAction: (name: string, args: unknown[]) => Promise<unknown>,
): AgentOsInspectorSource {
	return {
		manifestQueryOptions: (actorId) =>
			queryOptions({
				queryKey: agentOsQueryKeys.manifest(actorId),
				queryFn: async () =>
					(await executeAction(
						"agentOsManifest",
						[],
					)) as AgentOsManifest | null,
			}),
		sessionsQueryOptions: (actorId) =>
			queryOptions({
				queryKey: agentOsQueryKeys.sessions(actorId),
				queryFn: async () =>
					(await executeAction(
						"listPersistedSessions",
						[],
					)) as SessionSummary[],
			}),
		transcriptQueryOptions: (actorId, sessionId) =>
			queryOptions({
				queryKey: agentOsQueryKeys.transcript(actorId, sessionId),
				queryFn: async () =>
					(await executeAction("getSessionEvents", [
						sessionId,
					])) as TranscriptEvent[],
			}),
		filesystemQueryOptions: (actorId) =>
			queryOptions({
				queryKey: agentOsQueryKeys.filesystem(actorId),
				queryFn: async () =>
					(await executeAction("readdirRecursive", ["/"])) as FsNode,
			}),
		fileContentQueryOptions: (actorId, path) =>
			queryOptions({
				queryKey: agentOsQueryKeys.fileContent(actorId, path),
				queryFn: async () =>
					(await executeAction("readFile", [
						path,
					])) as FileContent | null,
			}),
		processesQueryOptions: (actorId) =>
			queryOptions({
				queryKey: agentOsQueryKeys.processes(actorId),
				queryFn: async () =>
					(await executeAction("listProcesses", [])) as ProcessInfo[],
			}),
		toolsQueryOptions: (actorId) =>
			queryOptions({
				queryKey: agentOsQueryKeys.tools(actorId),
				queryFn: async () =>
					(await executeAction("agentOsTools", [])) as ToolsData,
			}),
		softwareQueryOptions: (actorId) =>
			queryOptions({
				queryKey: agentOsQueryKeys.software(actorId),
				queryFn: async () =>
					(await executeAction(
						"agentOsSoftware",
						[],
					)) as SoftwareBundle[],
			}),
		mountsQueryOptions: (actorId) =>
			queryOptions({
				queryKey: agentOsQueryKeys.mounts(actorId),
				queryFn: async () =>
					(await executeAction("agentOsMounts", [])) as MountInfo[],
			}),
		connectionsQueryOptions: (actorId) =>
			queryOptions({
				queryKey: agentOsQueryKeys.connections(actorId),
				queryFn: async () =>
					(await executeAction(
						"agentOsConnections",
						[],
					)) as ConnInfo[],
			}),
		metadataQueryOptions: (actorId) =>
			queryOptions({
				queryKey: agentOsQueryKeys.metadata(actorId),
				queryFn: async () =>
					(await executeAction(
						"agentOsMetadata",
						[],
					)) as AgentOsMetadata,
			}),
	};
}

const fixtureSource = createFixtureAgentOsSource();

/**
 * Picks the active source. The prototype always uses fixtures; the live source
 * is the seam for wiring real `executeAction` data later.
 */
export function useAgentOsSource(): AgentOsInspectorSource {
	return fixtureSource;
}

/**
 * Detects whether an actor is an agentOS actor (and returns its manifest).
 * Safe to call under the inspector guard — it reads fixtures, never
 * `useActorInspector()`. Returns `null`/`undefined` for non-agentOS actors.
 */
export function useAgentOsManifest(actorId: ActorId) {
	const source = useAgentOsSource();
	return useQuery(source.manifestQueryOptions(actorId));
}

const AgentOsInspectorContext = createContext<AgentOsInspectorSource | null>(
	null,
);

export function AgentOsInspectorProvider({
	children,
}: {
	children: ReactNode;
}) {
	const source = useAgentOsSource();
	return (
		<AgentOsInspectorContext.Provider value={source}>
			{children}
		</AgentOsInspectorContext.Provider>
	);
}

export function useAgentOsInspector(): AgentOsInspectorSource {
	const ctx = useContext(AgentOsInspectorContext);
	if (!ctx) {
		throw new Error(
			"useAgentOsInspector must be used within an AgentOsInspectorProvider",
		);
	}
	return ctx;
}
