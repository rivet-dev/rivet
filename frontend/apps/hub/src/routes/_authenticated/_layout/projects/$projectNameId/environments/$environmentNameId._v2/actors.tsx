import {
	ActorFeature,
	ActorNotFound,
	ActorsActorDetails,
	ActorsActorEmptyDetails,
	ActorsListFiltersSchema,
	ActorsListPreview,
	currentActorAtom,
	pickActorListFilters,
} from "@rivet-gg/components/actors";
import { useEnvironment } from "@/domains/project/data/environment-context";
import { useProject } from "@/domains/project/data/project-context";
import * as Layout from "@/domains/project/layouts/servers-layout";
import { actorBuildsCountQueryOptions } from "@/domains/project/queries";
import { useSuspenseQuery } from "@tanstack/react-query";
import {
	createFileRoute,
	type ErrorComponentProps,
	useRouter,
} from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { z } from "zod";
import { GettingStarted } from "@rivet-gg/components/actors";
import { useAtomValue } from "jotai";
import { useDialog } from "@/hooks/use-dialog";
import { ErrorComponent } from "@/components/error-component";
import { ActorsProvider } from "@/domains/project/components/actors/actors-provider";
function Actor() {
	const navigate = Route.useNavigate();
	const { tab } = Route.useSearch();

	const actor = useAtomValue(currentActorAtom);

	if (!actor) {
		return (
			<ActorNotFound
				features={[
					ActorFeature.Config,
					ActorFeature.Logs,
					ActorFeature.State,
					ActorFeature.Connections,
				]}
			/>
		);
	}

	return (
		<ActorsActorDetails
			actor={actor}
			tab={tab}
			onTabChange={(tab) => {
				navigate({
					to: ".",
					search: (old) => ({ ...old, tab }),
				});
			}}
		/>
	);
}

const FIXED_TAGS = {
	framework: "actor-core",
};
function Content() {
	const { nameId: projectNameId } = useProject();
	const { nameId: environmentNameId } = useEnvironment();
	const { actorId, modal, ...restSearch } = Route.useSearch();

	const CreateActorDialog = useDialog.CreateActor.Dialog;
	const GoToActorDialog = useDialog.GoToActor.Dialog;
	const router = useRouter();
	const navigate = Route.useNavigate();

	function handleOpenChange(open: boolean) {
		router.navigate({
			to: ".",
			search: (old) => ({
				...old,
				modal: !open ? undefined : modal,
			}),
		});
	}

	const filters = pickActorListFilters(restSearch);

	return (
		<ActorsProvider
			projectNameId={projectNameId}
			environmentNameId={environmentNameId}
			actorId={actorId}
			fixedTags={FIXED_TAGS}
			{...filters}
		>
			<ActorsListPreview>
				{actorId ? (
					<Actor />
				) : (
					<ActorsActorEmptyDetails
						features={[
							ActorFeature.Config,
							ActorFeature.Logs,
							ActorFeature.State,
							ActorFeature.Connections,
						]}
					/>
				)}
			</ActorsListPreview>

			<CreateActorDialog
				dialogProps={{
					open: modal === "create-actor",
					onOpenChange: handleOpenChange,
				}}
			/>
			<GoToActorDialog
				onSubmit={(actorId) => {
					navigate({
						to: ".",
						search: (old) => ({
							...old,
							actorId,
							modal: undefined,
						}),
					});
				}}
				dialogProps={{
					open: modal === "go-to-actor",
					onOpenChange: handleOpenChange,
				}}
			/>
		</ActorsProvider>
	);
}

function ProjectActorsRoute() {
	const { nameId: projectNameId } = useProject();
	const { nameId: environmentNameId } = useEnvironment();
	const { tags, createdAt, destroyedAt } = Route.useSearch();

	const { data } = useSuspenseQuery({
		...actorBuildsCountQueryOptions({
			projectNameId,
			environmentNameId,
		}),
		refetchInterval: (query) =>
			(query.state.data?.builds.length || 0) > 0 ? false : 2000,
	});

	if (data === 0 && !tags && !createdAt && !destroyedAt) {
		return <GettingStarted />;
	}

	return (
		<div className="flex flex-col max-w-full w-full h-full bg-card">
			<Content />
		</div>
	);
}

const searchSchema = z
	.object({
		actorId: z.string().optional(),
		tab: z.string().optional(),
	})
	.merge(ActorsListFiltersSchema);

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/_v2/actors",
)({
	validateSearch: zodValidator(searchSchema),
	staticData: {
		layout: "v2",
	},
	component: ProjectActorsRoute,
	pendingComponent: () => (
		<div className="p-4">
			<Layout.Content.Skeleton />
		</div>
	),
	errorComponent(props: ErrorComponentProps) {
		return (
			<div className="p-4">
				<div className="max-w-5xl mx-auto">
					<ErrorComponent {...props} />
				</div>
			</div>
		);
	},
});
