import type { Rivet } from "@rivet-gg/api";
import {
	Dd,
	Dl,
	Dt,
	Flex,
	Skeleton,
	formatDuration,
} from "@rivet-gg/components";
import { Suspense } from "react";
import { ActorBuild } from "./actor-build";
import { ActorObjectInspector } from "./console/actor-inspector";

export interface ActorRuntimeProps
	extends Omit<Rivet.actors.Actor, "createTs" | "startTs" | "destroyTs"> {
	createTs: Date | undefined;
	startTs: Date | undefined;
	destroyTs: Date | undefined;
	projectNameId: string;
	environmentNameId: string;
}

export function ActorRuntime({
	projectNameId,
	environmentNameId,
	lifecycle,
	runtime,
	resources,
}: ActorRuntimeProps) {
	return (
		<div className="px-4 mt-4 mb-4 ">
			<div className="flex gap-1 items-center mb-2">
				<h3 className=" font-semibold">Runtime</h3>
			</div>
			<Flex gap="2" direction="col" className="text-xs">
				<Dl>
					<Dt>Kill timeout</Dt>
					<Dd>{formatDuration(lifecycle.killTimeout || 0)}</Dd>
					<Dt>Resources</Dt>
					<Dd>
						{resources.cpu / 1000} CPU cores, {resources.memory} MB
						RAM
					</Dd>
					<Dt>Arguments</Dt>
					<Dd>
						<ActorObjectInspector data={runtime.arguments} />
					</Dd>
					<Dt>Environment</Dt>
					<Dd>
						<ActorObjectInspector data={runtime.environment} />
					</Dd>
					<Dt>Build</Dt>
					<Dd>
						<Suspense
							fallback={
								<Skeleton className="w-full h-32 col-span-2" />
							}
						>
							<ActorBuild
								projectNameId={projectNameId}
								environmentNameId={environmentNameId}
								buildId={runtime.build}
							/>
						</Suspense>
					</Dd>
				</Dl>
			</Flex>
		</div>
	);
}
