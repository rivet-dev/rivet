import { EnvironmentSelect } from "@/domains/project/components/environment-select";
import {
	environmentByIdQueryOptions,
	projectByIdQueryOptions,
	projectQueryOptions,
} from "@/domains/project/queries";
import { useSuspenseQuery } from "@tanstack/react-query";
import { Link, useNavigate } from "@tanstack/react-router";
import { useContext } from "react";
import { Fragment } from "react/jsx-runtime";
import { NavItem } from "../header/nav-item";
import { MobileBreadcrumbsContext } from "./mobile-breadcrumbs";
import { ProjectBreadcrumb } from "./project-breadcrumb";
import { Separator } from "./separator";

interface EnvironmentBreadcrumbProps {
	environmentNameId: string;
	projectNameId: string;
}

export function EnvironmentBreadcrumb({
	environmentNameId,
	projectNameId,
}: EnvironmentBreadcrumbProps) {
	const {
		data: { gameId: projectId },
	} = useSuspenseQuery(projectByIdQueryOptions(projectNameId));

	const {
		data: { namespaceId: environmentId, displayName },
	} = useSuspenseQuery(
		environmentByIdQueryOptions({ projectId, environmentNameId }),
	);

	const {
		data: { namespaces: environments },
	} = useSuspenseQuery(projectQueryOptions(projectId));

	const navigate = useNavigate();

	const handleEnvironmentChange = (environmentId: string) => {
		const environmentNameId = environments.find(
			(env) => env.namespaceId === environmentId,
		)?.nameId;

		if (!environmentNameId) return;
		navigate({
			to: "/projects/$projectNameId/environments/$environmentNameId",
			params: { projectNameId, environmentNameId },
		});
	};
	const isMobile = useContext(MobileBreadcrumbsContext);

	const Element = isMobile ? NavItem : Fragment;

	return (
		<>
			<ProjectBreadcrumb projectNameId={projectNameId} />
			<Separator />
			<Element>
				<Link
					to="/projects/$projectNameId/environments/$environmentNameId"
					params={{ projectNameId, environmentNameId }}
					className="flex items-center gap-2"
				>
					{displayName}
				</Link>
				<EnvironmentSelect
					variant="discrete"
					projectId={projectId}
					value={environmentId}
					onCreateClick={() =>
						navigate({
							to: ".",
							search: { modal: "create-environment" },
						})
					}
					showCreateEnvironment
					onValueChange={handleEnvironmentChange}
				/>
			</Element>
		</>
	);
}
