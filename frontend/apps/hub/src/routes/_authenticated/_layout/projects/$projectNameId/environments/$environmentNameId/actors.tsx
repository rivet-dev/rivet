import { GetStarted } from "@/components/get-started";
import { ActorsListPreview } from "@/domains/project/components/actors/actors-list-preview";
import * as Layout from "@/domains/project/layouts/servers-layout";
import { actorBuildsCountQueryOptions } from "@/domains/project/queries";
import { Icon, faActors } from "@rivet-gg/icons";
import { useSuspenseQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { useMemo } from "react";
import { z } from "zod";

function ProjectActorsRoute() {
	const {
		environment: { nameId: environmentNameId },
		project: { nameId: projectNameId },
	} = Route.useRouteContext();
	const { actorId, tags, showDestroyed } = Route.useSearch();
	const tagsRecord = useMemo(() => Object.fromEntries(tags || []), [tags]);

	const { data } = useSuspenseQuery(
		actorBuildsCountQueryOptions({
			projectNameId,
			environmentNameId,
		}),
	);

	if (data === 0 && !tags && showDestroyed === undefined) {
		return (
			<div className="w-full h-full flex flex-col justify-center">
				<div className="flex flex-col justify-center my-8">
					<Icon icon={faActors} className="text-6xl mx-auto my-4" />
					<h3 className="text-center font-bold text-xl max-w-md mb-2 mx-auto">
						Deploy your first Actor
					</h3>
					<p className="text-center text-muted-foreground max-w-sm mx-auto">
						Install Rivet to get started or use an existing template
						to get started.
					</p>
				</div>
				<GetStarted />
			</div>
		);
	}

	return (
		<div className="flex flex-col h-[calc(100vh-6.5rem)] bg-card -mx-4 -my-4">
			<ActorsListPreview
				projectNameId={projectNameId}
				environmentNameId={environmentNameId}
				actorId={actorId}
				tags={tagsRecord}
				showDestroyed={showDestroyed ?? true}
			/>
		</div>
	);
}

const searchSchema = z.object({
	actorId: z.string().optional(),
	tab: z.string().optional(),

	tags: z.array(z.tuple([z.string(), z.string()])).optional(),
	showDestroyed: z.boolean().optional(),
});

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/actors",
)({
	validateSearch: zodValidator(searchSchema),
	staticData: {
		layout: "actors",
	},
	component: ProjectActorsRoute,
	pendingComponent: Layout.Content.Skeleton,
});
