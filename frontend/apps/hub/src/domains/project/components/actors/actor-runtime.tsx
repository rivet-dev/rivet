import type { Rivet } from "@rivet-gg/api";
import {
	Code,
	CopyArea,
	Dd,
	Dl,
	Dt,
	Flex,
	Grid,
	Skeleton,
	formatDuration,
} from "@rivet-gg/components";
import { Suspense } from "react";
import { Fragment } from "react/jsx-runtime";
import { ActorBuild } from "./actor-build";

export interface ActorRuntimeProps
	extends Omit<Rivet.actor.Actor, "createTs" | "startTs" | "destroyTs"> {
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
		<div className="border mt-4 px-4 py-4 rounded-md relative">
			<p className="inline-block bg-card w-auto absolute -top-0 left-3 font-semibold px-0.5 -translate-y-1/2">
				Runtime
			</p>
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
						{runtime.arguments?.length === 0 ? (
							<p>No arguments provided.</p>
						) : (
							<Code>{runtime.arguments?.join(" ")}</Code>
						)}
					</Dd>
					<Dt>Environment</Dt>
					<Dd>
						{Object.keys(runtime.environment || {}).length === 0 ? (
							<p>No environment variables set.</p>
						) : (
							<Grid columns="2" gap="2">
								{Object.entries(runtime.environment || {}).map(
									([name, value]) => (
										<Fragment key={name}>
											<CopyArea
												variant="discrete"
												value={name}
											/>
											<CopyArea
												variant="discrete"
												value={value}
											/>
										</Fragment>
									),
								)}
							</Grid>
						)}
					</Dd>
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
				</Dl>
			</Flex>
		</div>
	);
}
