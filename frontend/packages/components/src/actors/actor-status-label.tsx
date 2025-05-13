import type { Actor, ActorAtom } from "./actor-context";
import { useAtomValue } from "jotai";
import { selectAtom } from "jotai/utils";
import type { ActorStatus } from "./actor-status-indicator";

export const ACTOR_STATUS_LABEL_MAP = {
	unknown: "Unknown",
	starting: "Starting",
	running: "Running",
	stopped: "Stopped",
	crashed: "Crashed",
} satisfies Record<ActorStatus, string>;

export const ActorStatusLabel = ({ status }: { status: ActorStatus }) => {
	return <span>{ACTOR_STATUS_LABEL_MAP[status]}</span>;
};

const selector = (a: Actor) => a.status;

export const AtomizedActorStatusLabel = ({
	actor,
}: {
	actor: ActorAtom;
}) => {
	const status = useAtomValue(selectAtom(actor, selector));
	return <ActorStatusLabel status={status} />;
};
