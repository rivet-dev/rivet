import type { Actor, ActorAtom } from "./actor-context";
import { useAtomValue } from "jotai";
import { selectAtom } from "jotai/utils";

interface ActorStatusLabelProps {
	actor: ActorAtom;
}

const selector = (a: Actor) => a.status;

export const ActorStatusLabel = ({ actor }: ActorStatusLabelProps) => {
	const status = useAtomValue(selectAtom(actor, selector));

	if (status === "running") {
		return <span>Running</span>;
	}

	if (status === "starting") {
		return <span>Starting</span>;
	}

	if (status === "crashed") {
		return <span>Crashed</span>;
	}

	if (status === "stopped") {
		return <span>Stopped</span>;
	}
};
