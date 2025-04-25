import { Ping, cn } from "@rivet-gg/components";
import type { Actor, ActorAtom } from "./actor-context";
import { useAtomValue } from "jotai";
import { selectAtom } from "jotai/utils";

type ActorStatus = "starting" | "running" | "stopped" | "crashed" | "unknown";

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

export const AtomizedActorStatusIndicator = ({
	actor,
}: {
	actor: ActorAtom;
}) => {
	const status = useAtomValue(selectAtom(actor, selector));
	return <ActorStatusIndicator status={status} />;
};

const selector = ({ status }: Actor) => status;

interface ActorStatusIndicatorProps {
	status: ReturnType<typeof getActorStatus>;
}

export const ActorStatusIndicator = ({ status }: ActorStatusIndicatorProps) => {
	if (status === "running") {
		return <Ping variant="success" className="relative right-auto" />;
	}

	return (
		<div
			className={cn("size-2 rounded-full", {
				"bg-blue-600 animate-pulse": status === "starting",
				"bg-destructive": status === "crashed",
				"bg-foreground/10": status === "stopped",
			})}
		/>
	);
};
