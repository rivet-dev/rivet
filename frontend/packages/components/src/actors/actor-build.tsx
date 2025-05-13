import { Flex, Dt, Dd, Dl, DiscreteCopyButton } from "@rivet-gg/components";
import { ActorTags } from "./actor-tags";
import { formatISO } from "date-fns";
import { actorBuildsAtom, type Actor, type ActorAtom } from "./actor-context";
import { selectAtom } from "jotai/utils";
import { useAtomValue } from "jotai";
import { useCallback } from "react";

const buildIdSelector = (a: Actor) => a.runtime?.build;

interface ActorBuildProps {
	actor: ActorAtom;
}

export function ActorBuild({ actor }: ActorBuildProps) {
	const buildId = useAtomValue(selectAtom(actor, buildIdSelector));

	const data = useAtomValue(
		selectAtom(
			actorBuildsAtom,
			useCallback(
				(builds) => {
					return builds.find((build) => build.id === buildId);
				},
				[buildId],
			),
		),
	);

	if (!data) {
		return null;
	}

	return (
		<div className="px-4 mt-4 mb-4 ">
			<div className="flex gap-1 items-center mb-2">
				<h3 className=" font-semibold">Build</h3>
			</div>
			<Flex gap="2" direction="col" className="text-xs">
				<Dl>
					<Dt>ID</Dt>
					<Dd>
						<DiscreteCopyButton size="xs" value={data.id}>
							{data.id}
						</DiscreteCopyButton>
					</Dd>
					<Dt>Created</Dt>
					<Dd>
						<DiscreteCopyButton
							size="xs"
							value={formatISO(data.createdAt)}
						>
							{formatISO(data.createdAt)}
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
								tags={data.tags}
							/>
						</Flex>
					</Dd>
				</Dl>
			</Flex>
		</div>
	);
}
