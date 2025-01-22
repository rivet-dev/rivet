import type { Rivet } from "@rivet-gg/api";
import { ClickToCopy, Dd, Dl, Dt, Flex, cn } from "@rivet-gg/components";
import { formatISO } from "date-fns";
import { ActorRegion } from "./actor-region";
import { ActorTags } from "./actor-tags";

export interface ActorGeneralProps
	extends Omit<Rivet.actor.Actor, "createTs" | "startTs" | "destroyTs"> {
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
						<ClickToCopy value={id}>
							<button type="button">{id.split("-")[0]}</button>
						</ClickToCopy>
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
						<ClickToCopy value={formatISO(createdAt)}>
							<button type="button">
								{formatISO(createdAt)}
							</button>
						</ClickToCopy>
					</Dd>
					<Dt>Destroyed</Dt>
					<Dd className={cn({ "text-muted-foreground": !destroyTs })}>
						<ClickToCopy
							value={destroyTs ? formatISO(destroyTs) : "n/a"}
						>
							<button type="button">
								{destroyTs ? formatISO(destroyTs) : "n/a"}
							</button>
						</ClickToCopy>
					</Dd>
				</Dl>
			</Flex>
		</div>
	);
}
