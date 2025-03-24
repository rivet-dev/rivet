import type { Rivet } from "@rivet-gg/api";
import {
	Button,
	Dd,
	Dl,
	DocsSheet,
	Dt,
	Flex,
	Skeleton,
	formatDuration,
} from "@rivet-gg/components";
import { Suspense } from "react";
import { ActorBuild } from "./actor-build";
import { ActorObjectInspector } from "./console/actor-inspector";
import { Icon, faBooks } from "@rivet-gg/icons";
import { ACTOR_FRAMEWORK_TAG_VALUE } from "./actor-tags";
import { toRecord } from "@/lib/utils";

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
	tags,
	resources,
}: ActorRuntimeProps) {
	return (
		<>
			<div className="px-4 mt-4 mb-4">
				<div className="flex gap-1 items-center mb-2">
					<h3 className=" font-semibold">Runtime</h3>
				</div>
				<Flex gap="2" direction="col" className="text-xs">
					<Dl>
						<Dt>Kill timeout</Dt>
						<Dd>{formatDuration(lifecycle.killTimeout || 0)}</Dd>
						{toRecord(tags).framework !==
						ACTOR_FRAMEWORK_TAG_VALUE ? (
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
							<ActorObjectInspector data={runtime.arguments} />
						</Dd>
						<Dt>Environment</Dt>
						<Dd>
							<ActorObjectInspector data={runtime.environment} />
						</Dd>
					</Dl>
				</Flex>
			</div>
			<div className="px-4 mt-4 mb-4 ">
				<div className="flex gap-1 items-center mb-2">
					<h3 className=" font-semibold">Build</h3>
				</div>

				<Suspense
					fallback={<Skeleton className="w-full h-32 col-span-2" />}
				>
					<ActorBuild
						projectNameId={projectNameId}
						environmentNameId={environmentNameId}
						buildId={runtime.build}
					/>
				</Suspense>
			</div>
			<div className="px-4 mt-4 mb-4 ">
				<div className="flex gap-1 items-center mb-2">
					<h3 className=" font-semibold">
						Durability & Rescheduling
					</h3>
					<DocsSheet
						title="Durability & Rescheduling"
						path="docs/durability"
					>
						<Button
							variant="outline"
							size="sm"
							startIcon={<Icon icon={faBooks} />}
						>
							Documentation
						</Button>
					</DocsSheet>
				</div>

				<Flex gap="2" direction="col" className="text-xs">
					<Dl>
						<Dt>Durable?</Dt>
						<Dd>{lifecycle.durable ? "Yes" : "No"}</Dd>
					</Dl>
				</Flex>
			</div>
		</>
	);
}
