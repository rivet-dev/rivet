import { GroupProjectSelect } from "@/domains/project/components/group-project-select";
import {
	projectByIdQueryOptions,
	projectQueryOptions,
	projectsQueryOptions,
} from "@/domains/project/queries";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { Fragment, useContext } from "react";
import { NavItem } from "../header/nav-item";
import { GroupBreadcrumb } from "./group-breadcrumb";
import { MobileBreadcrumbsContext } from "./mobile-breadcrumbs";
import { Separator } from "./separator";

interface ProjectBreadcrumbProps {
	projectNameId: string;
}

export function ProjectBreadcrumb({ projectNameId }: ProjectBreadcrumbProps) {
	const {
		data: { gameId: projectId },
	} = useSuspenseQuery(projectByIdQueryOptions(projectNameId));
	const { data } = useSuspenseQuery(projectQueryOptions(projectId));
	const { data: projects } = useSuspenseQuery(projectsQueryOptions());

	const navigate = useNavigate();
	const handleProjectChange = (projectId: string) => {
		const projectNameId = projects.find(
			(project) => project.gameId === projectId,
		)?.nameId;

		if (!projectNameId) return;

		navigate({
			to: "/projects/$projectNameId",
			params: { projectNameId },
		});
	};

	const isMobile = useContext(MobileBreadcrumbsContext);

	const Element = isMobile ? NavItem : Fragment;

	return (
		<>
			<GroupBreadcrumb groupId={data.developerGroupId} />
			<Separator />
			<Element>
				<Link
					to="/projects/$projectNameId"
					params={{ projectNameId }}
					className="flex items-center gap-2"
				>
					{data.displayName}
				</Link>
				{projects.length > 1 ? (
					<GroupProjectSelect
						variant="discrete"
						showCreateProject
						onCreateClick={() =>
							navigate({
								to: ".",
								search: {
									modal: "create-project",
									groupId: data.developerGroupId,
								},
							})
						}
						groupId={data.developerGroupId}
						value={projectId}
						onValueChange={handleProjectChange}
					/>
				) : null}
			</Element>
		</>
	);
}
