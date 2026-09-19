import {
	faRefresh,
	faSparkles,
	faTriangleExclamation,
	Icon,
} from "@rivet-gg/icons";
import { Button } from "@/components/ui/button";
import type { InspectorFailure } from "./mcp-error";

interface McpErrorPanelProps {
	failure: InspectorFailure;
	retrying: boolean;
	onRetry?: () => void;
	onAsk?: () => void;
	asked: boolean;
}

export function McpErrorPanel({
	failure,
	retrying,
	onRetry,
	onAsk,
	asked,
}: McpErrorPanelProps) {
	const qualified =
		failure.group && failure.code
			? `${failure.group}.${failure.code}`
			: undefined;
	return (
		<div className="flex h-full min-h-0 items-center justify-center p-6">
			<div className="w-full max-w-md overflow-hidden rounded-2xl border border-foreground/10 bg-card shadow-sm">
				<div className="flex gap-3.5 px-5 pt-5 pb-4">
					<div className="flex size-9 shrink-0 items-center justify-center rounded-full bg-destructive/10 text-destructive">
						<Icon icon={faTriangleExclamation} className="size-4" />
					</div>
					<div className="min-w-0 flex-1">
						<h2 className="text-sm font-semibold tracking-tight">
							{failure.title}
						</h2>
						<p className="mt-1 text-sm leading-relaxed text-muted-foreground">
							{failure.message}
						</p>
						<p className="mt-2.5 text-xs leading-relaxed text-muted-foreground">
							{failure.hint}
						</p>
						{qualified ? (
							<code className="mt-3 inline-block rounded-md bg-foreground/[0.06] px-1.5 py-0.5 font-mono text-[11px] text-muted-foreground">
								{qualified}
							</code>
						) : null}
					</div>
				</div>
				<div className="flex flex-wrap items-center gap-2 border-t border-foreground/10 bg-foreground/[0.02] px-5 py-3">
					{onRetry ? (
						<Button
							size="sm"
							variant="secondary"
							isLoading={retrying}
							startIcon={<Icon icon={faRefresh} />}
							onClick={onRetry}
						>
							Try again
						</Button>
					) : null}
					{onAsk ? (
						<Button
							size="sm"
							variant="ghost"
							disabled={asked}
							startIcon={<Icon icon={faSparkles} />}
							onClick={onAsk}
						>
							{asked ? "Sent to chat" : "Ask in chat"}
						</Button>
					) : null}
				</div>
			</div>
		</div>
	);
}
