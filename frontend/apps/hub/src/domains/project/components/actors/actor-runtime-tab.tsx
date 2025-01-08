import type { Rivet } from "@rivet-gg/api";
import {
	Code,
	CopyArea,
	CopyButton,
	Dd,
	Dl,
	Dt,
	Flex,
	Grid,
	ScrollArea,
	SmallText,
	WithTooltip,
	formatDuration,
} from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Fragment } from "react/jsx-runtime";
import { actorBuildQueryOptions } from "../../queries";
import { ActorTags } from "./actor-tags";

interface ActorRuntimeTabProps
	extends Omit<Rivet.actor.Actor, "createTs" | "startTs" | "destroyTs"> {
	createTs: Date | undefined;
	startTs: Date | undefined;
	destroyTs: Date | undefined;
	projectNameId: string;
	environmentNameId: string;
}

export function ActorRuntimeTab({
	projectNameId,
	environmentNameId,
	lifecycle,
	runtime,
	resources,
}: ActorRuntimeTabProps) {
	const { data } = useSuspenseQuery(
		actorBuildQueryOptions({
			projectNameId,
			environmentNameId,
			buildId: runtime.build,
		}),
	);

	return (
		<ScrollArea className="overflow-auto h-full px-4 my-2">
			<Flex gap="2" direction="col">
				<Dl>
					<Dt>Kill timeout</Dt>
					<Dd>{formatDuration(lifecycle.killTimeout || 0)}</Dd>
					<Dt>Resources</Dt>
					<Dd>
						{resources.cpu / 1000} CPU cores, {resources.memory} MB
						RAM
					</Dd>
					{data ? (
						<>
							<Dt id={runtime.build}>Build</Dt>
							<Dd />
							<div
								aria-describedby={runtime.build}
								className="col-span-2"
							>
								<Dl className="ml-5">
									<Dt>Id</Dt>
									<Dd>
										<WithTooltip
											content={data.id}
											trigger={
												<CopyButton value={data.id}>
													<button type="button">
														{data.id.split("-")[0]}
													</button>
												</CopyButton>
											}
										/>
									</Dd>
									<Dt>Created At</Dt>
									<Dd>{data.createdAt.toLocaleString()}</Dd>
									<Dt>Tags</Dt>
									<Dd>
										{Object.keys(data.tags).length > 0 ? (
											<ActorTags tags={data.tags} />
										) : (
											"None"
										)}
									</Dd>
								</Dl>
							</div>
						</>
					) : (
						<>
							<Dt>Build</Dt>
							<Dd>Unknown</Dd>
						</>
					)}
					<Dt>Arguments</Dt>
					<Dd>
						{runtime.arguments?.length === 0 ? (
							<SmallText>No arguments provided.</SmallText>
						) : (
							<Code>{runtime.arguments?.join(" ")}</Code>
						)}
					</Dd>
					<Dt>Environment</Dt>
					<Dd>
						{Object.keys(runtime.environment || {}).length === 0 ? (
							<SmallText>No environment variables set.</SmallText>
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
				</Dl>
			</Flex>
		</ScrollArea>
	);
}
