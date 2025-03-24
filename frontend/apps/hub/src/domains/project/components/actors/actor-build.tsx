import { useSuspenseQuery } from "@tanstack/react-query";
import { actorBuildQueryOptions } from "../../queries";
import { Flex, Dt, Dd, Dl, DiscreteCopyButton } from "@rivet-gg/components";
import { ActorTags } from "./actor-tags";
import { formatISO } from "date-fns";

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

	return (
		<Flex gap="2" direction="col" className="text-xs">
			<Dl>
				<Dt>Name</Dt>
				<Dd>
					<DiscreteCopyButton size="xs" value={data.name}>
						{data.name}
					</DiscreteCopyButton>
				</Dd>
				<Dt>ID</Dt>
				<Dd>
					<DiscreteCopyButton size="xs" value={data.id}>
						{data.id}
					</DiscreteCopyButton>
				</Dd>
				<Dt>Created</Dt>
				<Dd>
					<DiscreteCopyButton
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
							tags={data.tags}
						/>
					</Flex>
				</Dd>
			</Dl>
		</Flex>
	);
}
