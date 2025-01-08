import type { Rivet } from "@rivet-gg/api";
import { CommandItem } from "@rivet-gg/components";
import { useCommandPanelNavigation } from "./command-panel-navigation-provider";

interface ProjectsCommandPanelItemsProps {
	projects: Rivet.game.GameSummary[];
	groupId: string;
}

export function ProjectsCommandPanelItems({
	projects,
}: ProjectsCommandPanelItemsProps) {
	const { changePage } = useCommandPanelNavigation();
	return (
		<>
			{projects.map((project) => (
				<CommandItem
					key={project.gameId}
					value={project.gameId}
					keywords={[project.displayName]}
					onSelect={() => {
						changePage({
							key: "project",
							params: { projectNameId: project.nameId },
						});
					}}
				>
					{project.displayName}
				</CommandItem>
			))}
		</>
	);
}
