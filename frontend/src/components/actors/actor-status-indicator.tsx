import { faMoon, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import type { ComponentPropsWithRef } from "react";
import { cn } from "@/components";
import { useDataProvider } from "./data-provider";
import type { ActorId, ActorStatus } from "./queries";

export const QueriedActorStatusIndicator = ({
	actorId,
	...props
}: {
	actorId: ActorId;
} & ComponentPropsWithRef<"span">) => {
	const { data: status = "unknown", isError } = useQuery(
		useDataProvider().actorStatusQueryOptions(actorId),
	);

	return (
		<ActorStatusIndicator
			status={isError ? "stopped" : status}
			{...props}
		/>
	);
};

interface ActorStatusIndicatorProps extends ComponentPropsWithRef<"span"> {
	status: ActorStatus | undefined;
}

export const ActorStatusIndicator = ({
	status,
	...props
}: ActorStatusIndicatorProps) => {
	if (status === "sleeping") {
		return (
			<Icon
				icon={faMoon}
				className={cn(
					"text-indigo-400 text-[10px]",
					props.className,
				)}
			/>
		);
	}

	return (
		<span
			{...props}
			className={cn(
				"size-2 rounded-full",
				{
					"bg-green-500": status === "running",
					"bg-blue-600 animate-pulse": status === "starting",
					"bg-destructive":
						status === "crashed" || status === "crash-loop",
					"bg-foreground/10": status === "stopped",
					"bg-primary": status === "pending",
					"bg-accent": status === "unknown",
				},
				props.className,
			)}
		/>
	);
};
