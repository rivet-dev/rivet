import { memo } from "react";
import type { ReplCommand } from "../worker/actor-worker-container";
import { ActorConsoleLogFormatted } from "./actor-console-log-formatted";
import { ActorConsoleMessage } from "./actor-console-message";
import { ActorObjectInspector } from "./actor-inspector";

type ActorConsoleLogProps = ReplCommand & {
	showTimestmaps: boolean;
};

export const ActorConsoleLog = memo((props: ActorConsoleLogProps) => {
	return (
		<>
			<ActorConsoleMessage
				timestamp={
					props.showTimestmaps ? props.inputTimestamp : undefined
				}
				variant={
					props.status !== "success" && props.status !== "error"
						? "input-pending"
						: "input"
				}
			>
				{"formatted" in props && props.formatted ? (
					<ActorConsoleLogFormatted {...props.formatted} />
				) : null}
			</ActorConsoleMessage>
			{"error" in props ? (
				<ActorConsoleMessage
					variant="error"
					timestamp={
						props.showTimestmaps ? props.outputTimestamp : undefined
					}
				>
					{props.error &&
					typeof props.error === "object" &&
					"toString" in props.error
						? props.error.toString()
						: JSON.stringify(props.error)}
				</ActorConsoleMessage>
			) : null}
			{props.logs?.map((log, index) => (
				<ActorConsoleMessage
					// biome-ignore lint/suspicious/noArrayIndexKey: static array
					key={index}
					variant={log.method}
					timestamp={props.showTimestmaps ? log.timestamp : undefined}
				>
					{log.data?.map((element, key) => {
						if (typeof element === "string") {
							return (
								<span
									// biome-ignore lint/suspicious/noArrayIndexKey: static array
									key={key}
								>
									{element}
								</span>
							);
						}
						return (
							<ActorObjectInspector
								// biome-ignore lint/suspicious/noArrayIndexKey: static array
								key={index}
								data={element}
							/>
						);
					})}
				</ActorConsoleMessage>
			))}
			{"result" in props ? (
				<ActorConsoleMessage
					variant="output"
					timestamp={
						props.showTimestmaps ? props.outputTimestamp : undefined
					}
				>
					{typeof props.result === "string" ? (
						props.result
					) : (
						<ActorObjectInspector data={props.result} />
					)}
				</ActorConsoleMessage>
			) : null}
		</>
	);
});
