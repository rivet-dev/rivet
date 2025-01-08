import { projectsByGroupQueryOptions } from "@/domains/project/queries";
import { CommandGroup } from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { ProjectsCommandPanelItems } from "../projects-command-panel-items";

export function AllProjectsProjectsCommandGroup() {
	const { data } = useSuspenseQuery(projectsByGroupQueryOptions());

	return (
		<CommandGroup heading="Projects">
			{data.map((group) => (
				<ProjectsCommandPanelItems
					key={group.groupId}
					groupId={group.groupId}
					projects={group.projects}
				/>
			))}
		</CommandGroup>
	);
}
