import {
	faCheck,
	faClaude,
	faCopy,
	faCursor,
	faOpenai,
	faRectangleTerminal,
	faSparkles,
	Icon,
	type IconProp,
} from "@rivet-gg/icons";
import { useState } from "react";
import { CopyTrigger } from "@/components/copy-area";
import { Button } from "@/components/ui/button";
import { cn } from "@/components/lib/utils";
import { AGENT_CLIENTS, type AgentId } from "./catalog";
import { SETUP_PROMPT } from "./model";

export const AGENT_ICONS: Record<AgentId, IconProp> = {
	"claude-code": faClaude,
	codex: faOpenai,
	cursor: faCursor,
	opencode: faRectangleTerminal,
};

/**
 * Cloudflare-style "connect your agent" button. One click copies the one-line
 * prompt; the file it points to installs skills, MCP, and CLI, then has the
 * agent ask the user what to build, start locally, deploy here, and build a
 * dashboard. The what-to-do-next text only appears after the prompt is copied.
 */
export function AgentConnect() {
	const [copied, setCopied] = useState(false);

	return (
		<div className="flex flex-col items-center gap-3">
			<CopyTrigger value={SETUP_PROMPT} onClick={() => setCopied(true)}>
				<Button
					variant="outline"
					size="lg"
					className={cn(
						"h-auto gap-3 rounded-full bg-card py-2 pl-5 pr-4 text-base font-medium shadow-sm hover:border-foreground/40",
						copied && "border-foreground",
					)}
				>
					<Icon icon={faSparkles} className="text-primary" />
					Connect your agent to Rivet
					<span className="flex items-center gap-1.5 text-foreground/80">
						{AGENT_CLIENTS.map((c) => (
							<span
								key={c.id}
								title={c.label}
								className="flex size-7 items-center justify-center rounded-md border border-border bg-muted/60"
							>
								<Icon icon={AGENT_ICONS[c.id]} />
							</span>
						))}
					</span>
					<Icon
						icon={copied ? faCheck : faCopy}
						className={cn(
							copied ? "text-primary" : "text-muted-foreground",
						)}
					/>
				</Button>
			</CopyTrigger>

			{copied ? (
				<p className="animate-in fade-in slide-in-from-top-2 text-center text-xs text-muted-foreground duration-500">
					Copied. Paste it into your agent — it sets up Rivet, asks
					what to build, runs it locally, then deploys here.
				</p>
			) : null}
		</div>
	);
}

export function Code({
	children,
	className,
}: {
	children: string;
	className?: string;
}) {
	return (
		<pre
			className={cn(
				"overflow-auto whitespace-pre-wrap rounded-lg border border-border bg-muted/50 p-3 text-left font-mono text-xs leading-relaxed text-foreground/90",
				className,
			)}
		>
			{children}
		</pre>
	);
}
