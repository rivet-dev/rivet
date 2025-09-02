import { useQuery } from "@tanstack/react-query";
import { formatISO } from "date-fns";
import { Dd, DiscreteCopyButton, Dl, Dt, Flex } from "@/components";
import { useManager } from "./manager-context";
import type { ActorId } from "./queries";

interface ActorBuildProps {
	actorId: ActorId;
}

export function ActorBuild({ actorId }: ActorBuildProps) {
	const { data } = useQuery(useManager().actorBuildQueryOptions(actorId));

	if (!data) {
		return null;
	}

	return (
		<div className="px-4 my-8">
			<div className="flex gap-1 items-center mb-2">
				<h3 className=" font-semibold">Build</h3>
			</div>
			<Flex gap="2" direction="col" className="text-xs">
				<Dl>
					<Dt>ID</Dt>
					<Dd>
						<DiscreteCopyButton
							size="xs"
							value={data.id}
							className="truncate"
						>
							{data.id}
						</DiscreteCopyButton>
					</Dd>
					<Dt>Created</Dt>
					<Dd>
						<DiscreteCopyButton
							className="truncate"
							size="xs"
							value={formatISO(data.createdAt)}
						>
							{formatISO(data.createdAt)}
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
								truncate={false}
								tags={data.tags}
							/>
						</Flex>
					</Dd>
				</Dl>
			</Flex>
		</div>
	);
}
