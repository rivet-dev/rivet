import { useQuery } from "@tanstack/react-query";
import { formatISO } from "date-fns";
import { cn, Dd, DiscreteCopyButton, Dl, Dt, Flex } from "@/components";
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
				<h3 className="mb-1 text-sm font-semibold">General</h3>
				<p className="mb-4 text-xs text-muted-foreground">
					Identity, status, and lifecycle timestamps for this instance.
				</p>
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
						<QueriedActorStatusAdditionalInfo actorId={actorId} />
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
					<Dt>Runner</Dt>
					<Dd
						className={cn({
							"text-muted-foreground": !runner,
						})}
					>
						{runner || "n/a"}
					</Dd>
					<Dt>Created</Dt>
					<Dd className={cn({ "text-muted-foreground": !createTs })}>
						<DiscreteCopyButton
							size="xs"
							value={createTs ? formatISO(createTs) : "n/a"}
							className="-mx-2 h-auto"
						>
							{createTs ? formatISO(createTs) : "n/a"}
						</DiscreteCopyButton>
					</Dd>
					<Dt>Pending Allocation</Dt>
					<Dd
						className={cn({
							"text-muted-foreground": !pendingAllocationTs,
						})}
					>
						<DiscreteCopyButton
							size="xs"
							value={
								pendingAllocationTs
									? formatISO(pendingAllocationTs)
									: "n/a"
							}
							className="-mx-2 h-auto"
						>
							{pendingAllocationTs
								? formatISO(pendingAllocationTs)
								: "n/a"}
						</DiscreteCopyButton>
					</Dd>
					<Dt>Connectable</Dt>
					<Dd
						className={cn({
							"text-muted-foreground": !connectableTs,
						})}
					>
						<DiscreteCopyButton
							size="xs"
							className="-mx-2 h-auto"
							value={
								connectableTs ? formatISO(connectableTs) : "n/a"
							}
						>
							{connectableTs ? formatISO(connectableTs) : "n/a"}
						</DiscreteCopyButton>
					</Dd>
					<Dt>Sleeping</Dt>
					<Dd className={cn({ "text-muted-foreground": !sleepTs })}>
						<DiscreteCopyButton
							size="xs"
							className="-mx-2 h-auto"
							value={sleepTs ? formatISO(sleepTs) : "n/a"}
						>
							{sleepTs ? formatISO(sleepTs) : "n/a"}
						</DiscreteCopyButton>
					</Dd>
					{destroyTs ? (
						<>
							<Dt>Destroyed</Dt>
							<Dd
								className={cn({
									"text-muted-foreground": !destroyTs,
								})}
							>
								<DiscreteCopyButton
									size="xs"
									className="-mx-2 h-auto"
									value={
										destroyTs ? formatISO(destroyTs) : "n/a"
									}
								>
									{destroyTs ? formatISO(destroyTs) : "n/a"}
								</DiscreteCopyButton>
							</Dd>
						</>
					) : null}
					<Versions actorId={actorId} />
				</Dl>
			</Flex>
			</section>
			<section className="border-t border-foreground/[0.06] pt-5">
				<h3 className="mb-1 text-sm font-semibold">Actions</h3>
				<p className="mb-3 text-xs text-muted-foreground">
					Manage the lifecycle of this actor instance.
				</p>
				<div className="flex gap-2">
					<ActorSleepButton actorId={actorId} />
					<ActorRescheduleButton actorId={actorId} />
					<ActorStopButton actorId={actorId} />
				</div>
			</section>
		</div>
	);
}

function Versions({ actorId }: { actorId: ActorId }) {
	const { data } = useQuery(
		useDataProvider().actorStatusQueryOptions(actorId),
	);

	const { data: metadata } = useQuery(
		useDataProvider().metadataQueryOptions(),
	);

	const inspector = useActorInspector();

	if (data === "running" && inspector.isInspectorAvailable) {
		return (
			<>
				<Dt>Runner version</Dt>
				<Dd>{metadata?.version}</Dd>
				<ActorVersions actorId={actorId} />
			</>
		);
	}

	return (
		<>
			<Dt>Runner version</Dt>
			<Dd>{metadata?.version}</Dd>
			<Dt>Actor version</Dt>
			<Dd>
				<span className="text-muted-foreground">n/a</span>
			</Dd>
		</>
	);
}

function ActorVersions({ actorId }: { actorId: ActorId }) {
	const inspector = useActorInspector();

	const { data: actorMetadata } = useQuery({
		...inspector.actorMetadataQueryOptions(actorId),
		enabled: inspector.isInspectorAvailable,
	});

	return (
		<>
			<Dt>Actor version</Dt>
			<Dd>
				{actorMetadata?.version || (
					<span className="text-muted-foreground">n/a</span>
				)}
			</Dd>
		</>
	);
}
