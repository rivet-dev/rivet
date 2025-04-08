import { Suspense } from "react";
import { ActorBuild } from "./actor-build";
import { ActorObjectInspector } from "./console/actor-inspector";
import { ACTOR_FRAMEWORK_TAG_VALUE } from "./actor-tags";
import {
	ActorFeature,
	currentActorFeaturesAtom,
	type Actor,
	type ActorAtom,
} from "./actor-context";
import { selectAtom } from "jotai/utils";
import { useAtomValue } from "jotai";
import { Dd, Dl, Dt } from "../ui/typography";
import { Flex } from "../ui/flex";
import { formatDuration } from "../lib/formatter";
import { toRecord } from "../lib/utils";
import { Skeleton } from "../ui/skeleton";
import equal from "fast-deep-equal";

const selector = (a: Actor) => ({
	lifecycle: a.lifecycle,
	resources: a.resources,
	runtime: a.runtime,
	tags: a.tags,
});

export interface ActorRuntimeProps {
	actor: ActorAtom;
}

export function ActorRuntime({ actor }: ActorRuntimeProps) {
	const { lifecycle, resources, runtime, tags } = useAtomValue(
		selectAtom(actor, selector, equal),
	);

	const features = useAtomValue(currentActorFeaturesAtom);

	return (
		<>
			{features.includes(ActorFeature.Runtime) && lifecycle && runtime ? (
				<div className="px-4 mt-4 mb-4">
					<div className="flex gap-1 items-center mb-2">
						<h3 className=" font-semibold">Runtime</h3>
					</div>
					<Flex gap="2" direction="col" className="text-xs">
						<Dl>
							<Dt>Kill timeout</Dt>
							<Dd>
								{formatDuration(lifecycle.killTimeout || 0)}
							</Dd>
							{toRecord(tags).framework !==
								ACTOR_FRAMEWORK_TAG_VALUE && resources ? (
								<>
									<Dt>Resources</Dt>
									<Dd>
										{resources.cpu / 1000} CPU cores,{" "}
										{resources.memory} MB RAM
									</Dd>
								</>
							) : null}
							<Dt>Arguments</Dt>
							<Dd>
								<ActorObjectInspector
									data={runtime.arguments}
								/>
							</Dd>
							<Dt>Environment</Dt>
							<Dd>
								<ActorObjectInspector
									data={runtime.environment}
								/>
							</Dd>

							<Dt>Durable</Dt>
							<Dd>{lifecycle.durable ? "Yes" : "No"}</Dd>
						</Dl>
					</Flex>
				</div>
			) : null}

			<Suspense
				fallback={<Skeleton className="w-full h-32 col-span-2" />}
			>
				<ActorBuild actor={actor} />
			</Suspense>
		</>
	);
}
