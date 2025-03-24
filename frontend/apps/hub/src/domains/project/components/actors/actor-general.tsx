import type { Rivet } from "@rivet-gg/api";
import { Dd, DiscreteCopyButton, Dl, Dt, Flex, cn } from "@rivet-gg/components";
import { formatISO } from "date-fns";
import { ActorRegion } from "./actor-region";
import { ActorTags } from "./actor-tags";

export interface ActorGeneralProps
	extends Omit<Rivet.actors.Actor, "createTs" | "startTs" | "destroyTs"> {
	createTs: Date | undefined;
	startTs: Date | undefined;
	destroyTs: Date | undefined;
	projectNameId: string;
	environmentNameId: string;
}

export function ActorGeneral({
	id,
	projectNameId,
	environmentNameId,
	createdAt,
	region,
	destroyTs,
	tags,
}: ActorGeneralProps) {
	return (
		<div className="px-4 mt-4 ">
			<h3 className="mb-2 font-semibold">General</h3>
			<Flex gap="2" direction="col" className="text-xs">
				<Dl>
					<Dt>Region</Dt>
					<Dd>
						<ActorRegion
							className="justify-start"
							showLabel
							projectNameId={projectNameId}
							environmentNameId={environmentNameId}
							regionId={region}
						/>
					</Dd>
					<Dt>ID</Dt>
					<Dd className="text-mono">
						<DiscreteCopyButton size="xs" value={id}>
							{id}
						</DiscreteCopyButton>
					</Dd>
					<Dt>Tags</Dt>
					<Dd>
						<Flex
							direction="col"
							gap="2"
							className="flex-1 min-w-0"
							w="full"
						>
							<ActorTags
								className="justify-start text-foreground"
								tags={tags}
							/>
						</Flex>
					</Dd>
					<Dt>Created</Dt>
					<Dd>
						<DiscreteCopyButton
							size="xs"
							value={formatISO(createdAt)}
						>
							{formatISO(createdAt)}
						</DiscreteCopyButton>
					</Dd>
					<Dt>Destroyed</Dt>
					<Dd className={cn({ "text-muted-foreground": !destroyTs })}>
						<DiscreteCopyButton
							size="xs"
							value={destroyTs ? formatISO(destroyTs) : "n/a"}
						>
							{destroyTs ? formatISO(destroyTs) : "n/a"}
						</DiscreteCopyButton>
					</Dd>
				</Dl>
			</Flex>
		</div>
	);
}
