import { Button, WithTooltip } from "@rivet-gg/components";
import { faXmark, Icon } from "@rivet-gg/icons";

import equal from "fast-deep-equal";
import { useAtomValue } from "jotai";
import { selectAtom } from "jotai/utils";
import type { Actor, ActorAtom, DestroyActorAtom } from "./actor-context";

const selector = (a: Actor) => ({
	destroyedAt: a.destroyedAt,
	destroy: a.destroy,
});

interface ActorStopButtonProps {
	actor: ActorAtom;
}

export function ActorStopButton({ actor }: ActorStopButtonProps) {
	const { destroy: destroyAtom, destroyedAt } = useAtomValue(
		selectAtom(actor, selector, equal),
	);

	if (destroyedAt || !destroyAtom) {
		return null;
	}

	return <Content destroy={destroyAtom} />;
}

function Content({ destroy: destroyAtom }: { destroy: DestroyActorAtom }) {
	const { destroy, isDestroying } = useAtomValue(destroyAtom);
	return (
		<WithTooltip
			trigger={
				<Button
					isLoading={isDestroying}
					variant="destructive"
					size="icon-sm"
					onClick={destroy}
				>
					<Icon icon={faXmark} />
				</Button>
			}
			content="Stop Actor"
		/>
	);
}
