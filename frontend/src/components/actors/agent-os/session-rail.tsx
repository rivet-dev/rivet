// The console's left rail: the agent's session list. Lifted out of the
// Transcript tab so it stays visible across all tabs (sessions are
// console-level; the other tabs are VM-global and ignore the selection).

import { cn, RelativeTime, ScrollArea } from "@/components";
import { AgentOsEmpty, type DotColor, StatusDot } from "./common";
import type { SessionStatus, SessionSummary } from "./types";

const SESSION_DOT: Record<SessionStatus, DotColor> = {
	running: "green",
	idle: "muted",
	error: "red",
};

export function SessionRail({
	sessions,
	selectedSessionId,
	onSelectSession,
	agentName,
	className,
}: {
	sessions: SessionSummary[];
	selectedSessionId: string | null;
	onSelectSession: (sessionId: string) => void;
	agentName?: string;
	className?: string;
}) {
	return (
		<div
			className={cn(
				"flex h-full w-64 shrink-0 flex-col border-r bg-card",
				className,
			)}
		>
			<div className="border-b px-3 py-2.5">
				<div className="truncate text-sm font-semibold">
					{agentName ?? "Sessions"}
				</div>
				<div className="text-[11px] uppercase tracking-wider text-muted-foreground">
					{sessions.length} session{sessions.length === 1 ? "" : "s"}
				</div>
			</div>
			<ScrollArea className="min-h-0 flex-1">
				{sessions.length === 0 ? (
					<AgentOsEmpty>No sessions yet.</AgentOsEmpty>
				) : (
					<div className="p-1.5">
						{sessions.map((session) => (
							<button
								key={session.sessionId}
								type="button"
								onClick={() =>
									onSelectSession(session.sessionId)
								}
								className={cn(
									"flex w-full items-center gap-2 rounded px-2 py-2 text-left transition-colors",
									session.sessionId === selectedSessionId
										? "bg-accent text-accent-foreground"
										: "hover:bg-accent/50",
								)}
							>
								<StatusDot
									color={SESSION_DOT[session.status]}
								/>
								<div className="min-w-0 flex-1">
									<div className="truncate font-mono text-xs">
										{session.sessionId}
									</div>
									<div className="text-[11px] text-muted-foreground">
										{session.agentType} ·{" "}
										<RelativeTime
											time={new Date(session.createdAt)}
										/>
									</div>
								</div>
							</button>
						))}
					</div>
				)}
			</ScrollArea>
		</div>
	);
}
