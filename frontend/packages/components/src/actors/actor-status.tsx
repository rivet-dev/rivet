import { cn } from "@rivet-gg/components";
import {
	ActorStatusIndicator,
	type ActorStatus as ActorStatusType,
	AtomizedActorStatusIndicator,
} from "./actor-status-indicator";
import {
	ActorStatusLabel,
	AtomizedActorStatusLabel,
} from "./actor-status-label";
import type { ActorAtom } from "./actor-context";

interface ActorStatusProps {
	className?: string;
	actor: ActorAtom;
}

export const AtomizedActorStatus = ({
	className,
	...props
}: ActorStatusProps) => {
	return (
		<div
			className={cn(
				"flex items-center gap-x-2 border rounded-full px-2.5 py-0.5",
				className,
			)}
		>
			<AtomizedActorStatusIndicator {...props} />
			<AtomizedActorStatusLabel {...props} />
		</div>
	);
};

export const ActorStatus = ({
	className,
	status,
}: {
	className?: string;
	status: ActorStatusType;
}) => {
	return (
		<div
			className={cn(
				"flex items-center gap-x-2 border rounded-full px-2.5 py-0.5",
				className,
			)}
		>
			<ActorStatusIndicator status={status} />
			<ActorStatusLabel status={status} />
		</div>
	);
};
