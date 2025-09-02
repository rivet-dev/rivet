import { useQuery } from "@tanstack/react-query";
import { formatISO } from "date-fns";
import { cn, Dd, DiscreteCopyButton, Dl, Dt, Flex } from "@/components";
import { ActorRegion } from "./actor-region";
import { ActorObjectInspector } from "./console/actor-inspector";
import { useManager } from "./manager-context";
import type { ActorId } from "./queries";

export interface ActorGeneralProps {
	actorId: ActorId;
}

export function ActorGeneral({ actorId }: ActorGeneralProps) {
	const {
		data: {
			region,
			keys,
			createdAt,
			destroyedAt,
			connectableAt,
			pendingAllocationAt,
			sleepingAt,
			crashPolicy,
		} = {},
	} = useQuery(useManager().actorGeneralQueryOptions(actorId));

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
							regionId={region}
						/>
					</Dd>
					<Dt>ID</Dt>
					<Dd className="text-mono">
						<DiscreteCopyButton size="xs" value={actorId}>
							{actorId}
						</DiscreteCopyButton>
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
					<Dt>Crash Policy</Dt>
					<Dd>{crashPolicy}</Dd>
					<Dt>Created</Dt>
					<Dd className={cn({ "text-muted-foreground": !createdAt })}>
						<DiscreteCopyButton
							size="xs"
							value={createdAt ? formatISO(createdAt) : "n/a"}
						>
							{createdAt ? formatISO(createdAt) : "n/a"}
						</DiscreteCopyButton>
					</Dd>
					<Dt>Pending Allocation</Dt>
					<Dd
						className={cn({
							"text-muted-foreground": !pendingAllocationAt,
						})}
					>
						<DiscreteCopyButton
							size="xs"
							value={
								pendingAllocationAt
									? formatISO(pendingAllocationAt)
									: "n/a"
							}
						>
							{pendingAllocationAt
								? formatISO(pendingAllocationAt)
								: "n/a"}
						</DiscreteCopyButton>
					</Dd>
					<Dt>Connectable</Dt>
					<Dd
						className={cn({
							"text-muted-foreground": !connectableAt,
						})}
					>
						<DiscreteCopyButton
							size="xs"
							value={
								connectableAt ? formatISO(connectableAt) : "n/a"
							}
						>
							{connectableAt ? formatISO(connectableAt) : "n/a"}
						</DiscreteCopyButton>
					</Dd>
					<Dt>Sleeping</Dt>
					<Dd
						className={cn({ "text-muted-foreground": !sleepingAt })}
					>
						<DiscreteCopyButton
							size="xs"
							value={sleepingAt ? formatISO(sleepingAt) : "n/a"}
						>
							{sleepingAt ? formatISO(sleepingAt) : "n/a"}
						</DiscreteCopyButton>
					</Dd>
					<Dt>Destroyed</Dt>
					<Dd
						className={cn({
							"text-muted-foreground": !destroyedAt,
						})}
					>
						<DiscreteCopyButton
							size="xs"
							value={destroyedAt ? formatISO(destroyedAt) : "n/a"}
						>
							{destroyedAt ? formatISO(destroyedAt) : "n/a"}
						</DiscreteCopyButton>
					</Dd>
				</Dl>
			</Flex>
		</div>
	);
}
