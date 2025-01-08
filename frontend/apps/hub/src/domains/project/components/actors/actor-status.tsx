import type { Rivet } from "@rivet-gg/api";
import { cn } from "@rivet-gg/components";
import { ActorStatusIndicator } from "./actor-status-indicator";
import { ActorStatusLabel } from "./actor-status-label";

interface ActorStatusProps extends Rivet.actor.Actor {
	className?: string;
}

export const ActorStatus = ({ className, ...props }: ActorStatusProps) => {
	return (
		<div className={cn("flex items-center gap-x-2", className)}>
			<ActorStatusIndicator {...props} />
			<ActorStatusLabel {...props} />
		</div>
	);
};
