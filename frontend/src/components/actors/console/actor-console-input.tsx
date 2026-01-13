import { useQuery } from "@tanstack/react-query";
import { useRef, useState } from "react";
import { Button, ScrollArea } from "@/components";
import { useActorInspector } from "../actor-inspector-context";
import type { ActorId } from "../queries";
import { useActorWorker } from "../worker/actor-worker-context";
import { ActorConsoleMessage } from "./actor-console-message";
import { ReplInput, type ReplInputRef, replaceCode } from "./repl-input";

interface ActorConsoleInputProps {
	actorId: ActorId;
}

export function ActorConsoleInput({ actorId }: ActorConsoleInputProps) {
	const worker = useActorWorker();

	const actorInspector = useActorInspector();
	const { data: rpcs = [] } = useQuery(
		actorInspector.actorRpcsQueryOptions(actorId),
	);

	const ref = useRef<ReplInputRef>(null);
	const [history, setHistory] = useState<string[]>([]);
	const [historyIndex, setHistoryIndex] = useState(-1);

	return (
		<div className="border-t w-full max-h-20 flex flex-col">
			<ScrollArea className="w-full flex-1">
				<ActorConsoleMessage variant="input" className="border-b-0">
					<ReplInput
						ref={ref}
						className="w-full"
						rpcs={rpcs}
						onRun={(code) => {
							if (code.trim()) {
								setHistory((prev) => [...prev, code]);
								setHistoryIndex(-1);
							}
							worker.run(code);
						}}
						onHistoryUp={() => {
							if (history.length === 0) return;
							const newIndex =
								historyIndex === -1
									? history.length - 1
									: Math.max(0, historyIndex - 1);
							setHistoryIndex(newIndex);
							if (ref.current?.view) {
								replaceCode(
									ref.current.view,
									history[newIndex],
								);
							}
						}}
						onHistoryDown={() => {
							if (historyIndex === -1) return;
							const newIndex = historyIndex + 1;
							if (newIndex >= history.length) {
								setHistoryIndex(-1);
								if (ref.current?.view) {
									replaceCode(ref.current.view, "");
								}
							} else {
								setHistoryIndex(newIndex);
								if (ref.current?.view) {
									replaceCode(
										ref.current.view,
										history[newIndex],
									);
								}
							}
						}}
					/>
				</ActorConsoleMessage>
				<div className="flex items-center mt-1 pb-1 px-1">
					<div className="flex flex-wrap gap-1">
						{rpcs.map((rpc) => (
							<Button
								variant="outline"
								size="xs"
								key={rpc}
								onClick={() => {
									if (!ref.current?.view) {
										return;
									}
									replaceCode(
										ref.current.view,
										`actor.${rpc}(`,
									);
								}}
								className="rounded-lg"
								startIcon={
									<span className="bg-secondary px-1 rounded-full">
										RPC
									</span>
								}
							>
								<span className="font-mono-console">
									actor.{rpc}(...)
								</span>
							</Button>
						))}
					</div>
				</div>
			</ScrollArea>
		</div>
	);
}
