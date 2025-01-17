import { CopyButton, Dd, Dl, Dt, WithTooltip } from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { useId } from "react";
import { actorBuildQueryOptions } from "../../queries";
import { ActorTags } from "./actor-tags";

interface ActorBuildProps {
	projectNameId: string;
	environmentNameId: string;
	buildId: string;
}

export function ActorBuild({
	projectNameId,
	environmentNameId,
	buildId,
}: ActorBuildProps) {
	const { data } = useSuspenseQuery(
		actorBuildQueryOptions({
			projectNameId,
			environmentNameId,
			buildId,
		}),
	);

	const id = useId();

	return (
		<div className="border mt-2 px-4 py-4 rounded-md relative col-span-2">
			<p
				id={id}
				className="inline-block bg-card w-auto absolute -top-0 left-3 font-semibold px-0.5 -translate-y-1/2"
			>
				Build
			</p>
			<div aria-describedby={id}>
				<Dl>
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
		</div>
	);
}
