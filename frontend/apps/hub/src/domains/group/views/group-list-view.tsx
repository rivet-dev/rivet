import { projectsByGroupQueryOptions } from "@/domains/project/queries";
import { Flex } from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Group } from "../components/group";
import { GroupCreateCard } from "../components/group-create-card";
import { NoGroupsAlert } from "../components/no-groups-alert";

export function GroupListView() {
	const { data } = useSuspenseQuery(projectsByGroupQueryOptions());
	return (
		<Flex direction="col">
			{data.length === 0 ? (
				<NoGroupsAlert />
			) : (
				<>
					{data.map((group) => (
						<Group key={group.groupId} {...group} />
					))}
					<GroupCreateCard />
				</>
			)}
		</Flex>
	);
}

GroupListView.Skeleton = () => {
	return (
		<Flex direction="col">
			<Group.Skeleton />
			<Group.Skeleton />
			<Group.Skeleton />
		</Flex>
	);
};
