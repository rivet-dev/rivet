import { cn } from "@/components";
import {
	ActorStatusIndicator,
	QueriedActorStatusIndicator,
} from "./actor-status-indicator";
import {
	ActorStatusLabel,
	QueriedActorStatusLabel,
} from "./actor-status-label";
import type { ActorId, ActorStatus as ActorStatusType } from "./queries";

interface ActorStatusProps {
	className?: string;
	actorId: ActorId;
}

export const QueriedActorStatus = ({
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
			<QueriedActorStatusIndicator {...props} />
			<QueriedActorStatusLabel {...props} />
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
