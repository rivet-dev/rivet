import { Ping, cn } from "@rivet-gg/components";
import type { Actor, ActorAtom } from "./actor-context";
import { useAtomValue } from "jotai";
import { selectAtom } from "jotai/utils";
import type { ComponentPropsWithRef } from "react";

export type ActorStatus =
	| "starting"
	| "running"
	| "stopped"
	| "crashed"
	| "unknown";

export function getActorStatus(
	actor: Pick<Actor, "createdAt" | "startedAt" | "destroyedAt">,
): ActorStatus {
	const { createdAt, startedAt, destroyedAt } = actor;

	if (createdAt && !startedAt && !destroyedAt) {
		return "starting";
	}

	if (createdAt && startedAt && !destroyedAt) {
		return "running";
	}

	if (createdAt && startedAt && destroyedAt) {
		return "stopped";
	}

	if (createdAt && !startedAt && destroyedAt) {
		return "crashed";
	}

	return "unknown";
}

interface AtomizedActorStatusIndicatorProps
	extends ComponentPropsWithRef<"span"> {
	actor: ActorAtom;
}

export const AtomizedActorStatusIndicator = ({
	actor,
	...props
}: AtomizedActorStatusIndicatorProps) => {
	const status = useAtomValue(selectAtom(actor, selector));
	return <ActorStatusIndicator status={status} {...props} />;
};

const selector = ({ status }: Actor) => status;

interface ActorStatusIndicatorProps extends ComponentPropsWithRef<"span"> {
	status: ReturnType<typeof getActorStatus>;
}

export const ActorStatusIndicator = ({
	status,
	...props
}: ActorStatusIndicatorProps) => {
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
				},
				props.className,
			)}
		/>
	);
};
