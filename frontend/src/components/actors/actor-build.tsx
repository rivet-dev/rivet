import { useQuery } from "@tanstack/react-query";
import { Dd, DiscreteCopyButton, Dl, Dt, Flex } from "@/components";
import { useDataProvider } from "./data-provider";
import type { ActorId } from "./queries";

interface ActorBuildProps {
	actorId: ActorId;
}

export function ActorBuild({ actorId }: ActorBuildProps) {
	const { data } = useQuery(
		useDataProvider().actorBuildQueryOptions(actorId),
	);

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
				</Dl>
			</Flex>
		</div>
	);
}
