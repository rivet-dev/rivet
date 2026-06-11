import {
	faFile,
	faRobot,
	faSparkles,
	faTerminal,
	faUser,
	faWrench,
	Icon,
} from "@rivet-gg/icons";
import type { ReactNode } from "react";
import { cn } from "@/components";
import { ActorObjectInspector } from "../console/actor-inspector";
import { formatBytes } from "./common";
import type { TranscriptEvent } from "./types";

function EventFrame({
	icon,
	label,
	meta,
	children,
}: {
	icon: typeof faUser;
	label: string;
	meta?: ReactNode;
	children?: ReactNode;
}) {
	return (
		<div className="border-b px-4 py-3">
			<div className="mb-1.5 flex items-center gap-2 text-[11px] uppercase tracking-wider text-muted-foreground">
				<Icon icon={icon} className="size-3 shrink-0" />
				<span className="font-medium">{label}</span>
				{meta != null ? (
					<span className="ml-auto normal-case tracking-normal">
						{meta}
					</span>
				) : null}
			</div>
			{children}
		</div>
	);
}

export function TranscriptEventView({ event }: { event: TranscriptEvent }) {
	switch (event.kind) {
		case "user":
			return (
				<EventFrame icon={faUser} label="User">
					<p className="whitespace-pre-wrap text-sm">{event.text}</p>
				</EventFrame>
			);
		case "assistant":
			return (
				<EventFrame icon={faRobot} label="Assistant">
					<p className="whitespace-pre-wrap text-sm">{event.text}</p>
				</EventFrame>
			);
		case "thinking":
			return (
				<EventFrame icon={faSparkles} label="Thinking">
					<p className="whitespace-pre-wrap text-sm italic text-muted-foreground">
						{event.text}
					</p>
				</EventFrame>
			);
		case "shell":
			return (
				<EventFrame
					icon={faTerminal}
					label="Shell"
					meta={
						<span
							className={cn(
								"font-mono text-xs",
								event.exitCode === 0
									? "text-muted-foreground"
									: "text-red-500",
							)}
						>
							exit {event.exitCode} · {event.durationMs} ms
						</span>
					}
				>
					<div className="font-mono text-xs">
						<span className="text-muted-foreground">$ </span>
						{event.command}
					</div>
					{event.output ? (
						<pre className="mt-1.5 whitespace-pre-wrap break-words rounded bg-muted/50 p-2 font-mono text-xs leading-relaxed text-muted-foreground">
							{event.output}
						</pre>
					) : null}
				</EventFrame>
			);
		case "tool":
			return (
				<EventFrame
					icon={faWrench}
					label={`Tool · ${event.tool}`}
					meta={
						<span className="font-mono text-xs text-muted-foreground">
							{event.latencyMs} ms
						</span>
					}
				>
					<div className="space-y-2">
						<ToolValue label="Input" value={event.input} />
						{event.error ? (
							<div>
								<div className="mb-1 text-[11px] uppercase tracking-wider text-muted-foreground">
									Error
								</div>
								<code className="font-mono text-xs text-red-500">
									{event.error}
								</code>
							</div>
						) : (
							<ToolValue label="Output" value={event.output} />
						)}
					</div>
				</EventFrame>
			);
		case "file_write":
			return (
				<EventFrame
					icon={faFile}
					label="File write"
					meta={
						<span className="font-mono text-xs text-muted-foreground">
							{formatBytes(event.bytes)}
						</span>
					}
				>
					<div className="font-mono text-xs">{event.path}</div>
				</EventFrame>
			);
		default: {
			// Exhaustiveness guard: adding a TranscriptEvent variant is a compile error.
			const _never: never = event;
			return _never;
		}
	}
}

function ToolValue({ label, value }: { label: string; value: unknown }) {
	return (
		<div>
			<div className="mb-1 text-[11px] uppercase tracking-wider text-muted-foreground">
				{label}
			</div>
			<div className="rounded bg-muted/50 p-2 text-xs">
				<ActorObjectInspector name={label.toLowerCase()} data={value} />
			</div>
		</div>
	);
}
