import { Dd, DiscreteCopyButton, Dl, Dt, Flex, cn } from "@rivet-gg/components";
import { formatISO } from "date-fns";
import { ActorRegion } from "./actor-region";
import { ActorTags } from "./actor-tags";
import type { Actor, ActorAtom } from "./actor-context";
import { useAtomValue } from "jotai";
import { selectAtom } from "jotai/utils";
import equal from "fast-deep-equal";

const selector = (a: Actor) => ({
	id: a.id,
	tags: a.tags,
	createdAt: a.createdAt,
	destroyedAt: a.destroyedAt,
	region: a.region,
});

export interface ActorGeneralProps {
	actor: ActorAtom;
}

export function ActorGeneral({ actor }: ActorGeneralProps) {
	const { id, tags, createdAt, destroyedAt, region } = useAtomValue(
		selectAtom(actor, selector, equal),
	);

	return (
		<div className="px-4 mt-4 ">
			<h3 className="mb-2 font-semibold">General</h3>
			<Flex gap="2" direction="col" className="text-xs">
				<Dl>
					<Dt>Region</Dt>
					<Dd>
						<ActorRegion
							className="justify-start"
							showLabel
							regionId={region}
						/>
					</Dd>
					<Dt>ID</Dt>
					<Dd className="text-mono">
						<DiscreteCopyButton size="xs" value={id}>
							{id}
						</DiscreteCopyButton>
					</Dd>
					<Dt>Tags</Dt>
					<Dd>
						<Flex
							direction="col"
							gap="2"
							className="flex-1 min-w-0"
							w="full"
						>
							<ActorTags
								className="justify-start text-foreground"
								truncate={false}
								tags={tags}
							/>
						</Flex>
					</Dd>
					<Dt>Created</Dt>
					<Dd className={cn({ "text-muted-foreground": !createdAt })}>
						<DiscreteCopyButton
							size="xs"
							value={createdAt ? formatISO(createdAt) : "n/a"}
						>
							{createdAt ? formatISO(createdAt) : "n/a"}
						</DiscreteCopyButton>
					</Dd>
					<Dt>Destroyed</Dt>
					<Dd
						className={cn({
							"text-muted-foreground": !destroyedAt,
						})}
					>
						<DiscreteCopyButton
							size="xs"
							value={destroyedAt ? formatISO(destroyedAt) : "n/a"}
						>
							{destroyedAt ? formatISO(destroyedAt) : "n/a"}
						</DiscreteCopyButton>
					</Dd>
				</Dl>
			</Flex>
		</div>
	);
}
