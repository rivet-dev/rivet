import { faMoon, Icon } from "@rivet-gg/icons";
import { useQuery } from "@tanstack/react-query";
import type { ComponentPropsWithRef } from "react";
import { cn, Ping } from "@/components";
import { useManager } from "./manager-context";
import type { ActorId, ActorStatus } from "./queries";

export const QueriedActorStatusIndicator = ({
	actorId,
	...props
}: {
	actorId: ActorId;
} & ComponentPropsWithRef<"span">) => {
	const { data: status, isError } = useQuery(
		useManager().actorStatusQueryOptions(actorId),
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
		return <Icon icon={faMoon} className="text-indigo-400" />;
	}

	if (status === "running") {
		return (
			<Ping
				variant="success"
				{...props}
				className={cn("relative right-auto", props.className)}
			/>
		);
	}

	return (
		<span
			{...props}
			className={cn(
				"size-2 rounded-full",
				{
					"bg-blue-600 animate-pulse": status === "starting",
					"bg-destructive": status === "crashed",
					"bg-foreground/10": status === "stopped",
					"bg-accent": status === "unknown",
				},
				props.className,
			)}
		/>
	);
};
