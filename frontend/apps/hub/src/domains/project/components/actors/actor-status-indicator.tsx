import type { Rivet } from "@rivet-gg/api";
import { Ping, cn } from "@rivet-gg/components";

export function getActorStatus(
	actor: Pick<Rivet.actors.Actor, "createdAt" | "startedAt" | "destroyedAt">,
) {
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

interface ActorStatusIndicatorProps
	extends Pick<
		Rivet.actors.Actor,
		"createdAt" | "startedAt" | "destroyedAt"
	> {}

export const ActorStatusIndicator = ({
	createdAt,
	startedAt,
	destroyedAt,
}: ActorStatusIndicatorProps) => {
	const status = getActorStatus({ createdAt, startedAt, destroyedAt });

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
