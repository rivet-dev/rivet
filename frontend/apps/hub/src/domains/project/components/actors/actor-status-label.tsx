import type { Rivet } from "@rivet-gg/api";

interface ActorStatusLabelProps extends Rivet.actor.Actor {}

export const ActorStatusLabel = ({
	createdAt,
	startedAt,
	destroyedAt,
}: ActorStatusLabelProps) => {
	const isStarting = createdAt && !startedAt && !destroyedAt;
	const isRunning = createdAt && startedAt && !destroyedAt;
	const isStopped = createdAt && startedAt && destroyedAt;
	const isCrashed = createdAt && !startedAt && destroyedAt;

	if (isRunning) {
		return <span>Running</span>;
	}

	if (isStarting) {
		return <span>Starting</span>;
	}

	if (isCrashed) {
		return <span>Crashed</span>;
	}

	if (isStopped) {
		return <span>Stopped</span>;
	}
};
