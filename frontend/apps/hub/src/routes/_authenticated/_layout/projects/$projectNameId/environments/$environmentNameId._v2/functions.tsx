import * as Layout from "@/domains/project/layouts/servers-layout";
import {
	projectActorsQueryOptions,
	routesQueryOptions,
	useDeleteRouteMutation,
} from "@/domains/project/queries";
import {
	useInfiniteQuery,
	usePrefetchInfiniteQuery,
	useSuspenseQuery,
} from "@tanstack/react-query";
import {
	createFileRoute,
	type ErrorComponentProps,
	Link,
} from "@tanstack/react-router";
import { zodValidator } from "@tanstack/zod-adapter";
import { z } from "zod";
import { ErrorComponent } from "@/components/error-component";
import {
	Button,
	Table,
	TableHeader,
	TableRow,
	TableHead,
	TableBody,
	TableCell,
	DiscreteCopyButton,
	toRecord,
	DropdownMenu,
	DropdownMenuTrigger,
	DropdownMenuContent,
	DropdownMenuItem,
	Text,
	H1,
} from "@rivet-gg/components";
import { Icon, faPlus, faEllipsisH } from "@rivet-gg/icons";
import { useEnvironment } from "@/domains/project/data/environment-context";
import { useProject } from "@/domains/project/data/project-context";
import { useDialog } from "@/hooks/use-dialog";

function ProjectFunctionsRoute() {
	const { projectNameId, environmentNameId } = Route.useParams();
	const { data: routes } = useSuspenseQuery(
		routesQueryOptions(Route.useParams()),
	);

	usePrefetchInfiniteQuery({
		...projectActorsQueryOptions({
			projectNameId,
			environmentNameId,
			includeDestroyed: false,
			tags: {},
		}),
		pages: 10,
	});

	const { data: actors } = useInfiniteQuery(
		projectActorsQueryOptions({
			projectNameId,
			environmentNameId,
			includeDestroyed: false,
			tags: {},
		}),
	);

	const navigate = Route.useNavigate();

	const { mutate: deleteRoute } = useDeleteRouteMutation();

	return (
		<>
			<Modals />

			<div className="max-w-5xl mx-auto my-8 flex justify-between items-center">
				<H1>Functions</H1>
				<div className="flex items-center gap-2">
					<Button
						variant="outline"
						startIcon={<Icon icon={faPlus} />}
						asChild
					>
						<Link
							to="."
							search={{ modal: "add-route" }}
							params={{
								projectNameId,
								environmentNameId,
							}}
						>
							Add Route
						</Link>
					</Button>
				</div>
			</div>

			<hr className="mb-4" />

			<div className="p-4">
				<div className="max-w-5xl mx-auto">
					<div className="border rounded-md">
						<Table>
							<TableHeader>
								<TableRow>
									<TableHead>Route</TableHead>
									<TableHead>Instances</TableHead>
									<TableHead />
								</TableRow>
							</TableHeader>
							<TableBody>
								{routes.length === 0 ? (
									<TableRow>
										<TableCell colSpan={3}>
											<Text className="text-center">
												There's no routes yet.
											</Text>
										</TableCell>
									</TableRow>
								) : null}
								{routes?.map((route) => (
									<TableRow key={route.id}>
										<TableCell>
											<DiscreteCopyButton
												value={`${route.hostname}${route.path}${route.routeSubpaths ? "/*" : ""}`}
											>
												{`${route.hostname}${route.path}${route.routeSubpaths ? "/*" : ""}`}
											</DiscreteCopyButton>
										</TableCell>
										<TableCell>
											{actors?.filter((actor) =>
												Object.entries(
													route.target.actors
														?.selectorTags || {},
												).some(([key, value]) => {
													return (
														toRecord(actor.tags)[
															key
														] === value
													);
												}),
											).length || 0}
										</TableCell>
										<TableCell>
											<DropdownMenu>
												<DropdownMenuTrigger asChild>
													<Button
														aria-haspopup="true"
														size="icon"
														variant="ghost"
													>
														<Icon
															className="size-4"
															icon={faEllipsisH}
														/>
														<span className="sr-only">
															Toggle menu
														</span>
													</Button>
												</DropdownMenuTrigger>
												<DropdownMenuContent align="end">
													<DropdownMenuItem
														onSelect={() =>
															navigate({
																to: ".",
																search: {
																	modal: "edit-route",
																	route: route.id,
																},
																params: {
																	projectNameId,
																	environmentNameId,
																},
															})
														}
													>
														Edit
													</DropdownMenuItem>
													<DropdownMenuItem
														onSelect={() => {
															deleteRoute({
																projectNameId,
																environmentNameId,
																routeId:
																	route.id,
															});
														}}
													>
														Delete
													</DropdownMenuItem>
												</DropdownMenuContent>
											</DropdownMenu>
										</TableCell>
									</TableRow>
								))}
							</TableBody>
						</Table>
					</div>
				</div>
			</div>
		</>
	);
}

function Modals() {
	const navigate = Route.useNavigate();
	const { gameId: projectId, nameId: projectNameId } = useProject();
	const { namespaceId: environmentId, nameId: environmentNameId } =
		useEnvironment();

	const { modal, route } = Route.useSearch();

	const EditRouteDialog = useDialog.EditRoute.Dialog;
	const CreateRouteDialog = useDialog.CreateRoute.Dialog;

	const handleOnOpenChange = (value: boolean) => {
		if (!value) {
			navigate({ search: { modal: undefined } });
		}
	};

	return (
		<>
			<EditRouteDialog
				projectNameId={projectNameId}
				environmentNameId={environmentNameId}
				// biome-ignore lint/style/noNonNullAssertion: at this point this should exist
				routeId={route!}
				dialogProps={{
					open: modal === "edit-route" && route !== undefined,
					onOpenChange: handleOnOpenChange,
				}}
			/>
			<CreateRouteDialog
				projectNameId={projectNameId}
				environmentNameId={environmentNameId}
				dialogProps={{
					open: modal === "add-route",
					onOpenChange: handleOnOpenChange,
				}}
			/>
		</>
	);
}

const searchSchema = z.object({
	modal: z.enum(["add-route", "edit-route"]).or(z.string()).optional(),
	route: z.string().optional(),
});

export const Route = createFileRoute(
	"/_authenticated/_layout/projects/$projectNameId/environments/$environmentNameId/_v2/functions",
)({
	validateSearch: zodValidator(searchSchema),
	staticData: {
		layout: "v2",
	},
	component: ProjectFunctionsRoute,
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
