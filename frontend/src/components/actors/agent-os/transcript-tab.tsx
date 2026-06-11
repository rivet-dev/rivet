import { useQuery } from "@tanstack/react-query";
import { ScrollArea } from "@/components";
import type { ActorId } from "../queries";
import { AgentOsEmpty, StatusDot } from "./common";
import { TranscriptEventView } from "./transcript-event";
import type { SessionSummary, TranscriptEvent } from "./types";
import { useAgentOsInspector } from "./use-agent-os-inspector";

// Event stream for the session selected in the console's left rail. The
// session list itself lives in `SessionRail` (console-level), not here.
export function TranscriptTab({
	session,
	events,
}: {
	session: SessionSummary | null;
	events: TranscriptEvent[];
}) {
	if (!session) {
		return (
			<AgentOsEmpty>
				Select a session to view its transcript.
			</AgentOsEmpty>
		);
	}

	return (
		<div className="flex h-full flex-col">
			<div className="border-b px-4 py-3">
				<div className="font-mono text-sm">{session.sessionId}</div>
				<div className="text-xs text-muted-foreground">
					{session.agentType} · {session.eventCount} event
					{session.eventCount === 1 ? "" : "s"}
				</div>
			</div>
			<ScrollArea className="min-h-0 flex-1">
				{events.length === 0 ? (
					<AgentOsEmpty>This session has no events yet.</AgentOsEmpty>
				) : (
					events.map((event) => (
						<TranscriptEventView key={event.seq} event={event} />
					))
				)}
			</ScrollArea>
			{session.status === "running" ? (
				<div className="flex items-center justify-center gap-2 border-t px-4 py-2 text-xs text-muted-foreground">
					<StatusDot color="green" className="animate-pulse" />
					Following · stream open
				</div>
			) : null}
		</div>
	);
}

export function TranscriptTabConnected({
	actorId,
	selectedSessionId,
}: {
	actorId: ActorId;
	selectedSessionId: string | null;
}) {
	const inspector = useAgentOsInspector();
	const { data: sessions = [] } = useQuery(
		inspector.sessionsQueryOptions(actorId),
	);
	const { data: events = [] } = useQuery({
		...inspector.transcriptQueryOptions(actorId, selectedSessionId ?? ""),
		enabled: !!selectedSessionId,
	});
	const session =
		sessions.find((s) => s.sessionId === selectedSessionId) ?? null;

	return (
		<TranscriptTab
			session={session}
			events={selectedSessionId ? events : []}
		/>
	);
}
