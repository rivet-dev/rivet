import { useQuery } from "@tanstack/react-query";
import { formatISO } from "date-fns";
import {
	Dd,
	DiscreteCopyButton,
	Dl,
	Dt,
	Flex,
	RelativeTime,
	WithTooltip,
} from "@/components";
import { useActorInspector } from "./actor-inspector-context";
import { ActorRegion } from "./actor-region";
import { QueriedActorStatus } from "./actor-status";
import { QueriedActorStatusAdditionalInfo } from "./actor-status-label";
import {
	ActorRescheduleButton,
	ActorSleepButton,
	ActorStopButton,
} from "./actor-stop-button";
import { ActorObjectInspector } from "./console/actor-inspector";
import { useDataProvider } from "./data-provider";
import type { ActorId } from "./queries";

export interface ActorGeneralProps {
	actorId: ActorId;
}

export function ActorGeneral({ actorId }: ActorGeneralProps) {
	const {
		data: {
			datacenter,
			keys,
			createTs,
			destroyTs,
			connectableTs,
			pendingAllocationTs,
			sleepTs,
			runner,
		} = {},
	} = useQuery(useDataProvider().actorGeneralQueryOptions(actorId));

	return (
		<div className="w-full px-6 py-5 flex flex-col gap-6">
			<section>
				<header className="mb-4 flex items-start justify-between gap-3">
					<div>
						<h3 className="mb-1 text-sm font-semibold">General</h3>
						<p className="text-xs text-muted-foreground">
							Identity, status, and lifecycle timestamps for this
							instance.
						</p>
					</div>
					<div className="flex items-center gap-2 shrink-0">
						<ActorSleepButton actorId={actorId} />
						<ActorRescheduleButton actorId={actorId} />
						<ActorStopButton actorId={actorId} />
					</div>
				</header>
				<Flex
					direction="col"
					className="text-xs [&_dl]:items-stretch [&_dt]:py-2 [&_dd]:py-2 [&_dt]:border-b [&_dd]:border-b [&_dt]:border-foreground/[0.06] [&_dd]:border-foreground/[0.06] [&_dt:last-of-type]:border-0 [&_dd:last-of-type]:border-0 [&_dt]:text-muted-foreground [&_dt]:font-normal [&_dd]:text-foreground"
				>
					<Dl>
						<Dt>Region</Dt>
						<Dd>
							<ActorRegion
								className="justify-start"
								showLabel
								regionId={datacenter}
							/>
						</Dd>
						<Dt>ID</Dt>
						<Dd className="text-mono">
							<DiscreteCopyButton
								size="xs"
								value={actorId}
								className="-mx-2 h-auto"
							>
								{actorId}
							</DiscreteCopyButton>
						</Dd>
						<Dt>Status</Dt>
						<Dd className="flex flex-col gap-1">
							<div>
								<div className="inline-block">
									<QueriedActorStatus actorId={actorId} />
								</div>
							</div>
							<QueriedActorStatusAdditionalInfo
								actorId={actorId}
							/>
						</Dd>
						<Dt>Keys</Dt>
						<Dd>
							<Flex
								direction="col"
								gap="2"
								className="flex-1 min-w-0"
								w="full"
							>
								<ActorObjectInspector
									data={keys}
									expandPaths={["$"]}
								/>
							</Flex>
						</Dd>
						{runner ? (
							<>
								<Dt>Runner</Dt>
								<Dd>{runner}</Dd>
							</>
						) : null}
						{createTs ? (
							<>
								<Dt>Created</Dt>
								<Dd>
									<TimestampValue ts={createTs} />
								</Dd>
							</>
						) : null}
						{pendingAllocationTs ? (
							<>
								<Dt>Pending Allocation</Dt>
								<Dd>
									<TimestampValue ts={pendingAllocationTs} />
								</Dd>
							</>
						) : null}
						{connectableTs ? (
							<>
								<Dt>Connectable</Dt>
								<Dd>
									<TimestampValue ts={connectableTs} />
								</Dd>
							</>
						) : null}
						{sleepTs ? (
							<>
								<Dt>Sleeping</Dt>
								<Dd>
									<TimestampValue ts={sleepTs} />
								</Dd>
							</>
						) : null}
						{destroyTs ? (
							<>
								<Dt>Destroyed</Dt>
								<Dd>
									<TimestampValue ts={destroyTs} />
								</Dd>
							</>
						) : null}
						<Versions actorId={actorId} />
					</Dl>
				</Flex>
			</section>
		</div>
	);
}

function TimestampValue({ ts }: { ts: Date }) {
	return (
		<DiscreteCopyButton
			size="xs"
			value={formatISO(ts)}
			className="-mx-2 h-auto"
		>
			<WithTooltip
				trigger={
					<span>
						<RelativeTime time={ts} />
					</span>
				}
				content={ts.toLocaleString()}
			/>
		</DiscreteCopyButton>
	);
}

function Versions({ actorId }: { actorId: ActorId }) {
	const inspector = useActorInspector();
	const { data: status } = useQuery(
		useDataProvider().actorStatusQueryOptions(actorId),
	);
	const { data: metadata } = useQuery(
		useDataProvider().metadataQueryOptions(),
	);

	const runnerVersion = metadata?.version;
	const showActorVersion =
		status === "running" && inspector.isInspectorAvailable;

	return (
		<>
			{runnerVersion ? (
				<>
					<Dt>Runner version</Dt>
					<Dd>{runnerVersion}</Dd>
				</>
			) : null}
			{showActorVersion ? <ActorVersions actorId={actorId} /> : null}
		</>
	);
}

function ActorVersions({ actorId }: { actorId: ActorId }) {
	const inspector = useActorInspector();

	if (!inspector.isInspectorAvailable || !inspector.rivetkitVersion)
		return null;

	return (
		<>
			<Dt>Actor version</Dt>
			<Dd>{inspector.rivetkitVersion}</Dd>
		</>
	);
}
