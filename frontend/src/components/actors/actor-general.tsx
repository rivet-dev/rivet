import { useQuery } from "@tanstack/react-query";
import { formatISO } from "date-fns";
import { cn, Dd, DiscreteCopyButton, Dl, Dt, Flex } from "@/components";
import { useActorInspector } from "./actor-inspector-context";
import { ActorRegion } from "./actor-region";
import { QueriedActorStatus } from "./actor-status";
import { QueriedActorStatusAdditionalInfo } from "./actor-status-label";
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
			crashPolicy,
			runner,
		} = {},
	} = useQuery(useDataProvider().actorGeneralQueryOptions(actorId));

	return (
		<div className="px-4 mt-4 mb-8">
			<h3 className="mb-2 font-semibold">General</h3>
			<Flex gap="2" direction="col" className="text-xs">
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
							className="-mx-2"
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
					<Dt>Crash Policy</Dt>
					<Dd
						className={cn({
							"text-muted-foreground": !crashPolicy,
						})}
					>
						{crashPolicy || "n/a"}
					</Dd>
					<Dt>Created</Dt>
					<Dd className={cn({ "text-muted-foreground": !createTs })}>
						<DiscreteCopyButton
							size="xs"
							value={createTs ? formatISO(createTs) : "n/a"}
							className="-mx-2"
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
							className="-mx-2"
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
							className="-mx-2"
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
							className="-mx-2"
							value={sleepTs ? formatISO(sleepTs) : "n/a"}
						>
							{sleepTs ? formatISO(sleepTs) : "n/a"}
						</DiscreteCopyButton>
					</Dd>
					<Dt>Destroyed</Dt>
					<Dd
						className={cn({
							"text-muted-foreground": !destroyTs,
						})}
					>
						<DiscreteCopyButton
							size="xs"
							className="-mx-2"
							value={destroyTs ? formatISO(destroyTs) : "n/a"}
						>
							{destroyTs ? formatISO(destroyTs) : "n/a"}
						</DiscreteCopyButton>
					</Dd>
					<Versions actorId={actorId} />
				</Dl>
			</Flex>
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
				<Dt>Runner version:</Dt>
				<Dd>{metadata?.version}</Dd>
				<ActorVersions actorId={actorId} />
			</>
		);
	}

	return (
		<>
			<Dt>Runner version:</Dt>
			<Dd>{metadata?.version}</Dd>
			<Dt>Actor version:</Dt>
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
			<Dt>Actor version:</Dt>
			<Dd>
				{actorMetadata?.version || (
					<span className="text-muted-foreground">n/a</span>
				)}
			</Dd>
		</>
	);
}
