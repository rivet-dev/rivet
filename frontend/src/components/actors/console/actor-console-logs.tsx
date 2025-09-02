import { useLayoutEffect, useRef } from "react";
import { ScrollArea } from "@/components";
import { useActorDetailsSettings } from "../actor-details-settings";
import { useActorReplCommands } from "../worker/actor-worker-context";
import { ActorConsoleLog } from "./actor-console-log";

export function ActorConsoleLogs() {
	const isScrolledToBottom = useRef(true);
	const ref = useRef<HTMLDivElement>(null);
	const commands = useActorReplCommands();

	const [settings] = useActorDetailsSettings();

	// biome-ignore lint/correctness/useExhaustiveDependencies: we want to run this effect on every commands change
	useLayoutEffect(() => {
		if (ref.current && isScrolledToBottom.current) {
			ref.current.scrollTop = ref.current.scrollHeight;
		}
	}, [commands]);

	return (
		<ScrollArea
			viewportRef={ref}
			viewportProps={{
				onScroll: (e) => {
					if (ref.current) {
						isScrolledToBottom.current =
							ref.current.scrollTop + ref.current.clientHeight >=
							ref.current.scrollHeight - 1;
					}
				},
			}}
			className="w-full flex-1"
		>
			{commands.map((log) => (
				<ActorConsoleLog
					{...log}
					key={log.key}
					showTimestmaps={settings.showTimestamps}
					inputTimestamp={log.inputTimestamp}
					outputTimestamp={log.outputTimestamp}
				/>
			))}
		</ScrollArea>
	);
}
