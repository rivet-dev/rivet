import { GroupAvatar } from "@/domains/group/components/group-avatar";
import { projectsByGroupQueryOptions } from "@/domains/project/queries";
import { CommandGroup, CommandItem } from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { useCommandPanelNavigation } from "../command-panel-navigation-provider";

export function AllGroupsCommandGroup() {
	const { data } = useSuspenseQuery(projectsByGroupQueryOptions());

	const { changePage } = useCommandPanelNavigation();

	return (
		<CommandGroup heading="Teams">
			{data.map((group) => {
				return (
					<CommandItem
						key={group.groupId}
						value={group.groupId}
						keywords={[group.displayName]}
						onSelect={() =>
							changePage({
								key: "group",
								params: { groupId: group.groupId },
							})
						}
					>
						<GroupAvatar
							className="mr-2 size-4"
							displayName={group.displayName}
							avatarUrl={group.avatarUrl}
						/>
						{group.displayName}
					</CommandItem>
				);
			})}
		</CommandGroup>
	);
}
