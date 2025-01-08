import * as Layout from "@/domains/project/layouts/project-layout";
import { modulesCategoriesQueryOptions } from "@/domains/project/queries";
import { ModulesStore } from "@rivet-gg/components";
import { useSuspenseQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";

function ProjectIdModules() {
	const { data } = useSuspenseQuery(modulesCategoriesQueryOptions());

	return <ModulesStore categories={data} includeModulesDocumentation />;
}

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/modules",
)({
	component: ProjectIdModules,
	pendingComponent: Layout.Root.Skeleton,
});
